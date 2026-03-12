import React, { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  Sparkles, 
  RefreshCw, 
  History, 
  Activity, 
  BrainCircuit, 
  ShieldCheck,
  ToggleLeft,
  ToggleRight,
  MessageCircle,
  Clock
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";

interface Expression {
  id: string;
  content: string;
  emotion: string;
  karma_refs: string[];
  created_at: string;
}

interface PipelineStatus {
  status: string;
  auto_expression: boolean;
  pending_expressions: number;
  last_insight: string;
  message_ja: string;
}

const ExpressionPipeline: React.FC = () => {
  const [expressions, setExpressions] = useState<Expression[]>([]);
  const [status, setStatus] = useState<PipelineStatus | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);

  const fetchStatus = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/expression/status`);
      if (res.ok) {
        const data = await res.json();
        setStatus(data);
      }
    } catch (e) {
      console.error("Failed to fetch expression status", e);
    }
  };

  const fetchExpressions = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/expression/list`);
      if (res.ok) {
        const data = await res.json();
        setExpressions(data);
      }
    } catch (e) {
      console.error("Failed to fetch expressions", e);
    }
  };

  const toggleAuto = async () => {
    if (!status) return;
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/expression/auto`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: !status.auto_expression })
      });
      if (res.ok) {
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to toggle auto-expression", e);
    }
  };

  const generateManually = async () => {
    setIsGenerating(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/expression/generate`, { method: "POST" });
      if (res.ok) {
        fetchExpressions();
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to generate expression", e);
    } finally {
      setIsGenerating(false);
    }
  };

  useEffect(() => {
    fetchStatus();
    fetchExpressions();
    const interval = setInterval(fetchStatus, 30000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="expression-pipeline" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-lg)' }}>
      {/* Header Section */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 'var(--space-md)' }}>
        <div>
          <h2 style={{ fontSize: '2rem', fontWeight: 800, background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))', WebkitBackgroundClip: 'text', backgroundClip: 'text', WebkitTextFillColor: 'transparent', display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
            <Sparkles color="var(--accent-cyan)" />
            AI Self-Expression Pipeline
          </h2>
          <p style={{ color: 'var(--text-muted)', marginTop: '0.25rem' }}>自律的な自己表現と感情発露の観測システム</p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)', background: 'var(--bg-glass-light)', border: '1px solid var(--border-glass)', padding: '0.5rem', borderRadius: 'var(--radius-md)', backdropFilter: 'blur(10px)' }}>
          <button 
            onClick={toggleAuto}
            className="card-hover"
            style={{ 
              display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem 1rem', borderRadius: 'var(--radius-sm)', 
              background: status?.auto_expression ? 'rgba(var(--accent-cyan-rgb), 0.1)' : 'transparent',
              color: status?.auto_expression ? 'var(--accent-cyan)' : 'var(--text-muted)',
              border: '1px solid currentColor', cursor: 'pointer', transition: 'all 0.2s', fontWeight: 600, fontSize: '0.85rem'
            }}
          >
            {status?.auto_expression ? <ToggleRight size={18} /> : <ToggleLeft size={18} />}
            Autonomous Mode: {status?.auto_expression ? "ON" : "OFF"}
          </button>
          
          <button 
            onClick={generateManually}
            disabled={isGenerating}
            className="primary-button"
            style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.6rem 1.2rem', opacity: isGenerating ? 0.5 : 1 }}
          >
            {isGenerating ? <RefreshCw className="animate-spin" size={18} /> : <Sparkles size={18} />}
            Generate Now
          </button>
        </div>
      </div>

      {/* Hero Status Card */}
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 'var(--space-md)' }}>
        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="main-panel ani-fade"
          style={{ padding: 'var(--space-lg)', position: 'relative', overflow: 'hidden', display: 'flex', flexDirection: 'column', justifyContent: 'center' }}
        >
          <div style={{ position: 'absolute', top: '10%', right: '5%', opacity: 0.05, pointerEvents: 'none' }}>
            <BrainCircuit size={160} />
          </div>
          
          <div style={{ position: 'relative', zIndex: 1, display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-cyan)', fontSize: '0.8rem', fontWeight: 700, letterSpacing: '0.1em' }}>
              <Activity size={16} />
              ACTIVE DELIBERATION
            </div>
            
            <h3 style={{ fontSize: '1.4rem', fontWeight: 600, color: 'var(--text-primary)' }}>Current Insight (Internal State)</h3>
            <p style={{ fontSize: '1.25rem', color: 'var(--text-secondary)', fontStyle: 'italic', lineHeight: 1.5 }}>
              &quot;{status?.last_insight || "Analyzing recent karma flow..."}&quot;
            </p>
            
            <div style={{ paddingTop: '1rem', display: 'flex', alignItems: 'center', gap: 'var(--space-md)', fontSize: '0.8rem', color: 'var(--text-muted)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                <ShieldCheck size={14} color="var(--accent-emerald)" />
                Constitutional Guard Active
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                <Clock size={14} />
                Next auto-sync in {Math.max(0, 5 - (expressions.length % 5))} cycles
              </div>
            </div>
          </div>
        </motion.div>

        <motion.div 
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="stat-card ani-fade"
          style={{ display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', textAlign: 'center', gap: '1rem' }}
        >
          <div style={{ width: '60px', height: '60px', borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center', background: status?.status === 'processing' ? 'rgba(var(--accent-cyan-rgb), 0.1)' : 'rgba(255,255,255,0.05)', color: status?.status === 'processing' ? 'var(--accent-cyan)' : 'var(--text-muted)' }}>
            {status?.status === 'processing' ? <RefreshCw className="animate-spin" size={32} /> : <Clock size={32} />}
          </div>
          <div>
            <div style={{ fontSize: '1.5rem', fontStyle: 'normal', fontWeight: 800, letterSpacing: '0.1em' }}>{status?.status?.toUpperCase() || "IDLE"}</div>
            <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginTop: '0.2rem' }}>PIPELINE STATE</div>
          </div>
          <div style={{ width: '100%', height: '4px', background: 'rgba(255,255,255,0.05)', borderRadius: '2px', overflow: 'hidden' }}>
            <motion.div 
              animate={{ width: status?.status === 'processing' ? '70%' : '100%' }}
              style={{ height: '100%', background: 'linear-gradient(90deg, var(--accent-cyan), var(--accent-purple))' }}
            />
          </div>
        </motion.div>
      </div>

      {/* Expressions Feed */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <h3 style={{ fontSize: '1.1rem', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <History size={18} color="var(--accent-purple)" />
          Self-Expression Artifacts
        </h3>
        
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(400px, 1fr))', gap: '1rem' }}>
          <AnimatePresence>
            {expressions.map((expr, idx) => (
              <motion.div
                key={expr.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: idx * 0.05 }}
                className="card-hover"
                style={{ background: 'var(--bg-glass-heavy)', borderRadius: 'var(--radius-md)', padding: 'var(--space-md)', position: 'relative', borderLeft: '4px solid var(--accent-cyan)' }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.75rem' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span style={{ padding: '0.1rem 0.5rem', background: 'rgba(var(--accent-cyan-rgb), 0.1)', color: 'var(--accent-cyan)', fontSize: '0.7rem', fontWeight: 700, borderRadius: '4px', border: '1px solid rgba(var(--accent-cyan-rgb), 0.2)' }}>
                      {expr.emotion.toUpperCase()}
                    </span>
                    <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>{expr.id.substring(0, 8)}</span>
                  </div>
                  <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                    <Clock size={12} />
                    {new Date(expr.created_at).toLocaleString()}
                  </span>
                </div>
                
                <p style={{ color: 'var(--text-primary)', lineHeight: 1.6, marginBottom: '1rem' }}>
                  {expr.content}
                </p>
                
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <div style={{ display: 'flex', gap: '0.15rem' }}>
                    {expr.karma_refs.map((ref, i) => (
                      <div key={i} title={ref} style={{ width: '18px', height: '18px', borderRadius: '50%', background: 'rgba(var(--accent-purple-rgb), 0.2)', border: '1px solid var(--accent-purple)', color: 'var(--accent-purple)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '9px' }}>
                        κ
                      </div>
                    ))}
                  </div>
                  <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                    Reflected {expr.karma_refs.length} Karma items
                  </span>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>

        {expressions.length === 0 && (
          <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)', background: 'var(--bg-glass-light)', borderRadius: 'var(--radius-lg)', border: '1px dashed var(--border-glass)' }}>
            <MessageCircle size={48} style={{ margin: '0 auto 1rem', opacity: 0.2 }} />
            <p>No self-expressions recorded yet.</p>
            <p style={{ fontSize: '0.8rem', marginTop: '0.5rem' }}>Trigger a manual generation or enable Autonomous Mode to start.</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default ExpressionPipeline;
