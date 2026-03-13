# ruri-v3 Embedding Server

Aiome のための日本語特化ローカル Embedding サーバーです。

## モデル

- **[ruri-v3-310m](https://huggingface.co/cl-nagoya/ruri-v3-310m)** (Apache 2.0)
- 名古屋大学 CL研究グループ製
- 日本語RAGベンチマーク（JMTEB）でトップクラスの精度
- 310Mパラメータ / 768次元 / 8192トークン対応

## セットアップ

```bash
cd tools/ruri-embed-server
pip install -r requirements.txt
python server.py
```

初回起動時に HuggingFace からモデルを自動ダウンロード（約600MB）します。

## API

### POST /embed
単一テキストの Embedding 生成。

```json
{
  "text": "AIを搭載した自律型OSです",
  "mode": "document"
}
```

`mode` は `"query"`, `"document"`, `"topic"`, `"semantic"` のいずれか。
ruri-v3 のプレフィックス方式に基づき自動的に付与されます。

### POST /embed/batch
複数テキストの一括 Embedding 生成。

### GET /health
ヘルスチェック。

## 環境変数

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `RURI_MODEL` | `cl-nagoya/ruri-v3-310m` | 使用するモデル |
| `RURI_PORT` | `8100` | リッスンポート |
