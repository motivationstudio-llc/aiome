import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Box,
  FileText,
  Code,
  Image as ImageIcon,
  Music,
  Share2,
  Database,
  Search,
  Download,
  Trash2,
  Calendar,
  User,
  Tag,
  Hash,
  Shield,
  Dna
} from "lucide-react";
import { API_BASE } from "../config";
import { getAuthHeaders } from "../lib/auth";

interface ArtifactFile {
  name: string;
  mime_type: string;
  size_bytes: number;
  hash: string;
}

interface ArtifactEdge {
  id: string;
  source_id: string;
  target_id: string;
  source_type: string;
  relation: string;
  metadata: any;
  created_at: string;
}

interface Artifact {
  id: string;
  title: string;
  category: string;
  tags: string[];
  created_by: string;
  dir_path: string;
  files: ArtifactFile[];
  karma_refs: string[];
  job_ref?: string;
  signature?: string;
  edges: ArtifactEdge[];
  created_at: string;
}

const ArtifactVault = () => {
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedArtifact, setSelectedArtifact] = useState<Artifact | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => {
      fetchArtifacts();
    }, 300);
    return () => clearTimeout(timer);
  }, [filter, searchTerm]);

  const fetchArtifacts = async () => {
    setLoading(true);
    try {
      let url = `${API_BASE}/api/artifacts?limit=50`;
      if (filter) url += `&category=${filter}`;
      if (searchTerm) url += `&q=${encodeURIComponent(searchTerm)}`;

      const res = await fetch(url, { headers: getAuthHeaders() });
      if (res.ok) {
        const data = await res.json();
        setArtifacts(data);
      }
    } catch (e) {
      console.error("Failed to fetch artifacts", e);
    } finally {
      setLoading(false);
    }
  };

  const getCategoryIcon = (category: string) => {
    switch (category) {
      case "report": return <FileText size={18} />;
      case "code": return <Code size={18} />;
      case "image": return <ImageIcon size={18} />;
      case "audio": return <Music size={18} />;
      case "expression": return <Share2 size={18} />;
      case "data": return <Database size={18} />;
      default: return <Box size={18} />;
    }
  };

  const deleteArtifact = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (!confirm("Are you sure you want to delete this artifact? This action is permanent and will purge physical files.")) return;

    try {
      const res = await fetch(`${API_BASE}/api/artifacts/${id}`, {
        method: "DELETE",
        headers: getAuthHeaders()
      });
      if (res.ok) {
        setArtifacts(prev => prev.filter(a => a.id !== id));
        if (selectedArtifact?.id === id) setSelectedArtifact(null);
      }
    } catch (e) {
      console.error("Failed to delete artifact", e);
    }
  };

  const filteredArtifacts = artifacts; // Filtered by API now (semantic support)

  return (
    <div className="vault-container">
      <div style={{ display: 'flex', gap: '1rem', marginBottom: '2rem', alignItems: 'center' }}>
        <div className="search-box">
          <Search size={18} />
          <input
            type="text"
            placeholder="Search artifacts or tags..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
          />
        </div>

        <div className="filter-chips">
          {['all', 'report', 'code', 'image', 'audio', 'expression', 'data'].map((cat) => (
            <button
              key={cat}
              className={`chip ${cat === (filter || 'all') ? 'active' : ''}`}
              onClick={() => setFilter(cat === 'all' ? null : cat)}
            >
              {cat === 'all' ? 'All' : cat.charAt(0).toUpperCase() + cat.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div style={{ padding: '4rem', textAlign: 'center' }}>
          <Box className="ani-pulse" size={48} color="var(--accent-cyan)" style={{ margin: '0 auto 1.5rem' }} />
          <p style={{ color: 'var(--text-secondary)' }}>Decrypting Artifact Vault...</p>
        </div>
      ) : (
        <div className="artifact-grid">
          {filteredArtifacts.map((artifact) => (
            <motion.div
              key={artifact.id}
              layoutId={artifact.id}
              className="artifact-card"
              onClick={() => setSelectedArtifact(artifact)}
            >
              <div className="card-header">
                <div className="category-tag">
                  {getCategoryIcon(artifact.category)}
                  <span>{artifact.category.toUpperCase()}</span>
                </div>
                <div className="timestamp">
                  {new Date(artifact.created_at).toLocaleDateString()}
                </div>
              </div>

              <h3 className="card-title">{artifact.title}</h3>

              <div className="card-meta">
                <div className="meta-item">
                  <User size={14} />
                  <span>{artifact.created_by}</span>
                </div>
                <div className="meta-item">
                  <Hash size={14} />
                  <span>{artifact.files.length} files</span>
                </div>
              </div>

              <div className="tag-list">
                {artifact.tags.map(t => <span key={t} className="tag">#{t}</span>)}
              </div>

              {artifact.signature && (
                <div className="signature-badge">
                  <Shield size={10} />
                  <span>VERIFIED</span>
                </div>
              )}

              <button
                className="delete-btn"
                onClick={(e) => deleteArtifact(e, artifact.id)}
                title="Purge Artifact"
              >
                <Trash2 size={14} />
              </button>
            </motion.div>
          ))}
        </div>
      )}

      {/* Details Modal */}
      <AnimatePresence>
        {selectedArtifact && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="modal-backdrop"
              onClick={() => setSelectedArtifact(null)}
            />
            <motion.div
              layoutId={selectedArtifact.id}
              className="artifact-modal"
            >
              <div className="modal-header">
                <div>
                  <div className="category-tag">
                    {getCategoryIcon(selectedArtifact.category)}
                    <span>{selectedArtifact.category.toUpperCase()}</span>
                  </div>
                  <h2>{selectedArtifact.title}</h2>
                </div>
                <button onClick={() => setSelectedArtifact(null)}>✕</button>
              </div>

              <div className="modal-content">
                <div className="file-section">
                  <h3>Files <span style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>({selectedArtifact.files.length})</span></h3>
                  <div className="file-list">
                    {selectedArtifact.files.map(file => (
                      <div key={file.name} className="file-item">
                        <div className="file-info">
                          <FileText size={16} color="var(--accent-cyan)" />
                          <div className="file-name-meta">
                            <span className="file-name">{file.name}</span>
                            <span className="file-size">{(file.size_bytes / 1024).toFixed(1)} KB</span>
                          </div>
                        </div>
                        <div className="file-actions">
                          <a
                            href={`${API_BASE}/api/artifacts/${selectedArtifact.id}/files/${file.name}`}
                            target="_blank"
                            rel="noreferrer"
                            className="icon-btn"
                          >
                            <Download size={16} />
                          </a>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="detail-sidebar">
                  <div className="detail-group">
                    <label><User size={14} /> Generator</label>
                    <p>{selectedArtifact.created_by}</p>
                  </div>
                  <div className="detail-group">
                    <label><Calendar size={14} /> Created</label>
                    <p>{new Date(selectedArtifact.created_at).toLocaleString()}</p>
                  </div>
                  <div className="detail-group">
                    <label><Tag size={14} /> Tags</label>
                    <div className="tag-list">
                      {selectedArtifact.tags.map(t => <span key={t} className="tag">#{t}</span>)}
                    </div>
                  </div>
                  {selectedArtifact.karma_refs.length > 0 && (
                    <div className="detail-group">
                      <label><Dna size={14} /> Karma Source</label>
                      <p style={{ fontSize: '0.7rem', color: 'var(--accent-purple)' }}>{selectedArtifact.karma_refs.join(", ")}</p>
                    </div>
                  )}
                  {selectedArtifact.signature && (
                    <div className="detail-group">
                      <label><Shield size={14} /> Audit Signature</label>
                      <p className="signature-text">{selectedArtifact.signature}</p>
                    </div>
                  )}

                  {selectedArtifact.edges && selectedArtifact.edges.length > 0 && (
                    <div className="detail-group">
                      <label><Hash size={14} /> Lineage (Provenance)</label>
                      <div className="edge-list">
                        {selectedArtifact.edges.map(edge => (
                          <div key={edge.id} className="edge-item">
                            <span className="edge-relation">{edge.relation}</span>
                            <span className="edge-target">{edge.target_id === selectedArtifact.id ? "from: " + edge.source_id.slice(0, 8) : "to: " + edge.target_id.slice(0, 8)}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      <style>{`
        .vault-container {
          padding: 1rem;
        }
        .search-box {
          display: flex;
          align-items: center;
          gap: 0.8rem;
          background: rgba(255,255,255,0.05);
          border: 1px solid rgba(255,255,255,0.1);
          border-radius: 12px;
          padding: 0 1rem;
          height: 48px;
          flex: 1;
        }
        .search-box input {
          background: transparent;
          border: none;
          color: white;
          width: 100%;
          outline: none;
        }
        .filter-chips {
          display: flex;
          gap: 0.5rem;
        }
        .chip {
          background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.05);
          color: var(--text-secondary);
          padding: 0.5rem 1rem;
          border-radius: 20px;
          font-size: 0.85rem;
          cursor: pointer;
          transition: all 0.2s;
        }
        .chip:hover {
          background: rgba(255,255,255,0.08);
        }
        .chip.active {
          background: var(--accent-cyan);
          color: black;
          font-weight: 600;
        }
        .artifact-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: 1.5rem;
        }
        .artifact-card {
          background: rgba(255,255,255,0.03);
          backdrop-filter: blur(10px);
          border: 1px solid rgba(255,255,255,0.05);
          border-radius: 16px;
          padding: 1.2rem;
          cursor: pointer;
          transition: transform 0.2s, background 0.2s;
          position: relative;
          overflow: hidden;
        }
        .artifact-card:hover {
          transform: translateY(-4px);
          background: rgba(255,255,255,0.06);
          border-color: rgba(0,242,255,0.3);
        }
        .card-header {
          display: flex;
          justify-content: space-between;
          margin-bottom: 1rem;
          align-items: center;
        }
        .category-tag {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          color: var(--accent-cyan);
          font-size: 0.7rem;
          font-weight: 700;
          letter-spacing: 0.05em;
        }
        .timestamp {
          font-size: 0.7rem;
          color: var(--text-muted);
        }
        .card-title {
          font-size: 1.1rem;
          margin-bottom: 1rem;
          color: white;
          line-height: 1.4;
        }
        .card-meta {
          display: flex;
          gap: 1rem;
          margin-bottom: 1rem;
        }
        .meta-item {
          display: flex;
          align-items: center;
          gap: 0.3rem;
          color: var(--text-secondary);
          font-size: 0.75rem;
        }
        .tag-list {
          display: flex;
          flex-wrap: wrap;
          gap: 0.4rem;
        }
        .tag {
          font-size: 0.7rem;
          color: var(--accent-purple);
          background: rgba(188,140,255,0.1);
          padding: 0.1rem 0.4rem;
          border-radius: 4px;
        }
        .signature-badge {
          position: absolute;
          bottom: 10px;
          right: -25px;
          background: var(--accent-rose);
          color: white;
          font-size: 0.6rem;
          padding: 0.2rem 2rem;
          transform: rotate(-45deg);
          display: flex;
          align-items: center;
          gap: 2px;
          font-weight: 800;
        }
        .delete-btn {
          position: absolute;
          top: 1.2rem;
          right: 1.2rem;
          background: rgba(255, 71, 87, 0.1);
          border: 1px solid rgba(255, 71, 87, 0.2);
          color: #ff4757;
          border-radius: 8px;
          padding: 0.4rem;
          cursor: pointer;
          opacity: 0;
          transition: all 0.2s;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .artifact-card:hover .delete-btn {
          opacity: 1;
        }
        .delete-btn:hover {
          background: #ff4757;
          color: white;
        }

        .modal-backdrop {
          position: fixed;
          inset: 0;
          background: rgba(0,0,0,0.8);
          z-index: 100;
        }
        .artifact-modal {
          position: fixed;
          top: 10%;
          left: 15%;
          right: 15%;
          bottom: 10%;
          background: #0a0c10;
          border: 1px solid rgba(255,255,255,0.1);
          border-radius: 24px;
          z-index: 101;
          display: flex;
          flex-direction: column;
          box-shadow: 0 20px 50px rgba(0,0,0,0.5);
        }
        .modal-header {
          padding: 1.5rem 2rem;
          border-bottom: 1px solid rgba(255,255,255,0.05);
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .modal-header h2 { margin-top: 0.4rem; }
        .modal-header button {
          background: transparent;
          border: none;
          color: var(--text-muted);
          font-size: 1.5rem;
          cursor: pointer;
        }
        .modal-content {
          flex: 1;
          display: grid;
          grid-template-columns: 1fr 300px;
          overflow: hidden;
        }
        .file-section {
          padding: 2rem;
          overflow-y: auto;
          border-right: 1px solid rgba(255,255,255,0.05);
        }
        .file-list {
          display: flex;
          flex-direction: column;
          gap: 0.8rem;
          margin-top: 1rem;
        }
        .file-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem;
          background: rgba(255,255,255,0.02);
          border-radius: 12px;
          border: 1px solid transparent;
          transition: all 0.2s;
        }
        .file-item:hover {
          background: rgba(255,255,255,0.04);
          border-color: rgba(255,255,255,0.1);
        }
        .file-info {
          display: flex;
          align-items: center;
          gap: 1rem;
        }
        .file-name-meta {
          display: flex;
          flex-direction: column;
        }
        .file-name {
          font-weight: 500;
          color: white;
        }
        .file-size {
          font-size: 0.75rem;
          color: var(--text-muted);
        }
        .icon-btn {
          color: var(--text-muted);
          transition: color 0.2s;
        }
        .icon-btn:hover {
          color: var(--accent-cyan);
        }
        .detail-sidebar {
          padding: 2rem;
          background: rgba(255,255,255,0.01);
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }
        .detail-group label {
          display: flex;
          align-items: center;
          gap: 0.4rem;
          font-size: 0.7rem;
          color: var(--text-muted);
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin-bottom: 0.5rem;
        }
        .detail-group p {
          color: var(--text-secondary);
          font-size: 0.9rem;
        }
        .signature-text {
          font-family: monospace;
          font-size: 0.65rem !important;
          word-break: break-all;
          background: rgba(0,0,0,0.3);
          padding: 0.5rem;
          border-radius: 6px;
        }
        .edge-list {
          display: flex;
          flex-direction: column;
          gap: 0.4rem;
        }
        .edge-item {
          font-size: 0.75rem;
          color: var(--text-secondary);
          background: rgba(255,255,255,0.03);
          padding: 0.4rem;
          border-radius: 4px;
          border-left: 2px solid var(--accent-cyan);
          display: flex;
          justify-content: space-between;
        }
        .edge-relation {
          color: var(--accent-cyan);
          font-weight: 600;
        }
        .edge-target {
          color: var(--text-muted);
          font-family: monospace;
        }
      `}</style>
    </div>
  );
};

export default ArtifactVault;
