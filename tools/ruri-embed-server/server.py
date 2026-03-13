"""
Aiome ruri-v3 Embedding Sidecar Server
=======================================
Japanese-optimized local embedding service using ruri-v3-310m.
Provides a REST API compatible with Aiome's EmbeddingProvider interface.

Model: cl-nagoya/ruri-v3-310m (Apache 2.0)
- 310M parameters, 768-dim output, 8192 token context
- Auto-downloads from HuggingFace on first run (~600MB)
"""

import os
import asyncio
import logging
from typing import List
from contextlib import asynccontextmanager
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field
from sentence_transformers import SentenceTransformer

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("ruri-embed")

MODEL_NAME = os.getenv("RURI_MODEL", "cl-nagoya/ruri-v3-310m")
PORT = int(os.getenv("RURI_PORT", "8100"))
HOST = os.getenv("RURI_HOST", "127.0.0.1")  # localhost only by default (security)

# Global model reference
model: SentenceTransformer = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Load model on startup, release on shutdown."""
    global model
    logger.info(f"🔮 Loading ruri-v3 model: {MODEL_NAME}")
    logger.info("   (First run will download ~600MB from HuggingFace)")

    import torch
    device = "cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu"
    logger.info(f"   Device: {device}")

    # Load model in a thread to avoid blocking the event loop
    loop = asyncio.get_event_loop()
    model = await loop.run_in_executor(None, lambda: SentenceTransformer(MODEL_NAME, device=device))
    logger.info(f"✅ ruri-v3 loaded successfully on {device}")
    yield
    logger.info("🔮 Shutting down ruri-v3 server")


app = FastAPI(
    title="Aiome ruri-v3 Embedding Server",
    version="1.0.0",
    lifespan=lifespan,
)


class EmbedRequest(BaseModel):
    text: str = Field(..., min_length=1, max_length=32768)
    mode: str = "document"  # "query", "document", "topic", or "semantic"


class EmbedResponse(BaseModel):
    embedding: List[float]
    dimensions: int
    model: str


class BatchEmbedRequest(BaseModel):
    texts: List[str] = Field(..., min_length=1, max_length=64)
    mode: str = "document"


class BatchEmbedResponse(BaseModel):
    embeddings: List[List[float]]
    dimensions: int
    model: str


# ruri-v3 prefix scheme:
# "" (empty)       → semantic meaning
# "トピック: "      → classification, clustering
# "検索クエリ: "    → retrieval queries
# "検索文書: "      → documents to be retrieved
PREFIX_MAP = {
    "semantic": "",
    "topic": "トピック: ",
    "query": "検索クエリ: ",
    "document": "検索文書: ",
}

VALID_MODES = frozenset(PREFIX_MAP.keys())


def add_prefix(text: str, mode: str) -> str:
    """Add ruri-v3 prefix based on embedding mode."""
    if mode not in VALID_MODES:
        mode = "document"
    prefix = PREFIX_MAP[mode]
    return f"{prefix}{text}" if prefix else text


@app.post("/embed", response_model=EmbedResponse)
async def embed_single(req: EmbedRequest):
    """Generate embedding for a single text."""
    if model is None:
        raise HTTPException(status_code=503, detail="Model not loaded")

    prefixed = add_prefix(req.text, req.mode)

    # Run model.encode in thread pool to avoid blocking the async event loop
    loop = asyncio.get_event_loop()
    vec = await loop.run_in_executor(
        None, lambda: model.encode(prefixed, convert_to_numpy=True).tolist()
    )

    return EmbedResponse(
        embedding=vec,
        dimensions=len(vec),
        model=MODEL_NAME,
    )


@app.post("/embed/batch", response_model=BatchEmbedResponse)
async def embed_batch(req: BatchEmbedRequest):
    """Generate embeddings for multiple texts."""
    if model is None:
        raise HTTPException(status_code=503, detail="Model not loaded")

    prefixed = [add_prefix(t, req.mode) for t in req.texts]

    # Run model.encode in thread pool to avoid blocking the async event loop
    loop = asyncio.get_event_loop()
    vecs = await loop.run_in_executor(
        None, lambda: model.encode(prefixed, convert_to_numpy=True).tolist()
    )

    return BatchEmbedResponse(
        embeddings=vecs,
        dimensions=len(vecs[0]) if vecs else 0,
        model=MODEL_NAME,
    )


@app.get("/health")
async def health():
    """Health check endpoint."""
    return {
        "status": "ok",
        "model": MODEL_NAME,
        "ready": model is not None,
    }


if __name__ == "__main__":
    import uvicorn
    logger.info(f"🚀 Starting ruri-v3 embedding server on {HOST}:{PORT}")
    uvicorn.run(app, host=HOST, port=PORT, log_level="info")
