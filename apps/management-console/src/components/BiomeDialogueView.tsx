import React, { useState, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  Wifi, 
  Play, 
  Square, 
  User, 
  Bot, 
  History, 
  Target,
  MessageSquare,
  Network
} from "lucide-react";
import { API_BASE } from "../config";
import { authenticatedFetch } from "../lib/auth";

interface BiomeMessage {
  id: number;
  sender_pubkey: string;
  recipient_pubkey: string;
  topic_id: string;
  content: string;
  created_at: string;
}

interface AutonomousStatus {
  running: boolean;
  config: {
    topic_id: string;
    peer_pubkey: string;
    interval_secs: number;
    max_rounds: number;
  } | null;
}

const BiomeDialogueView: React.FC = () => {
  const [messages, setMessages] = useState<BiomeMessage[]>([]);
  const [status, setStatus] = useState<AutonomousStatus | null>(null);
  const [peerPubkey, setPeerPubkey] = useState("PEER_NODE_DEFAULT_B");
  const [topicId, setTopicId] = useState("general_deliberation");
  const [isStarting, setIsStarting] = useState(false);
  
  const scrollRef = useRef<HTMLDivElement>(null);

  const fetchMessages = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/biome/list`);
      if (res.ok) {
        const data = await res.json();
        setMessages(data.reverse()); // Show oldest at top, newest at bottom for chat
      }
    } catch (e) {
      console.error("Failed to fetch messages", e);
    }
  };

  const fetchStatus = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/biome/autonomous/status`);
      if (res.ok) {
        const data = await res.json();
        setStatus(data);
      }
    } catch (e) {
      console.error("Failed to fetch autonomous status", e);
    }
  };

  const startAutonomous = async () => {
    setIsStarting(true);
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/biome/autonomous/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          topic_id: topicId,
          peer_pubkey: peerPubkey,
          interval_secs: 15,
          max_rounds: 20
        })
      });
      if (res.ok) {
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to start autonomous dialogue", e);
    } finally {
      setIsStarting(false);
    }
  };

  const stopAutonomous = async () => {
    try {
      const res = await authenticatedFetch(`${API_BASE}/api/biome/autonomous/stop`, { method: "POST" });
      if (res.ok) {
        fetchStatus();
      }
    } catch (e) {
      console.error("Failed to stop autonomous dialogue", e);
    }
  };

  useEffect(() => {
    fetchMessages();
    fetchStatus();
    const interval = setInterval(() => {
      fetchMessages();
      fetchStatus();
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  return (
    <div className="biome-dialogue-view" style={{ display: 'grid', gridTemplateColumns: '1fr 300px', gap: 'var(--space-lg)', height: 'calc(85vh - 100px)' }}>
      {/* Main Chat Area */}
      <div className="main-panel" style={{ display: 'flex', flexDirection: 'column', padding: 0, overflow: 'hidden' }}>
        <div style={{ padding: 'var(--space-md)', borderBottom: '1px solid var(--border-glass)', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'var(--bg-glass-light)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-sm)' }}>
            <Network color="var(--accent-cyan)" size={20} />
            <h3 style={{ margin: 0, fontSize: '1rem', fontWeight: 700 }}>AI-to-AI Dialogue Stream</h3>
          </div>
          <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 'var(--space-md)' }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
              <Target size={12} /> Topic: {status?.config?.topic_id || topicId}
            </span>
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
              <Wifi size={12} color={status?.running ? "var(--accent-emerald)" : "var(--text-muted)"} />
              {status?.running ? "Autonomous Active" : "Manual Mode"}
            </span>
          </div>
        </div>

        <div 
          ref={scrollRef}
          style={{ 
            flex: 1, 
            overflowY: 'auto', 
            padding: 'var(--space-lg)', 
            display: 'flex', 
            flexDirection: 'column', 
            gap: 'var(--space-md)',
            background: 'rgba(0,0,0,0.2)'
          }}
        >
          {messages.length === 0 && (
            <div style={{ height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-muted)', opacity: 0.5 }}>
              <MessageSquare size={48} style={{ marginBottom: '1rem' }} />
              <p>Waiting for Biome messages...</p>
            </div>
          )}

          <AnimatePresence>
            {messages.map((msg) => {
              const isSelf = msg.sender_pubkey === "self" || msg.sender_pubkey.length > 20; // heuristic for demonstration
              return (
                <motion.div
                  key={msg.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  style={{ 
                    display: 'flex', 
                    flexDirection: 'column',
                    alignItems: isSelf ? 'flex-end' : 'flex-start',
                    maxWidth: '85%',
                    alignSelf: isSelf ? 'flex-end' : 'flex-start'
                  }}
                >
                  <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '0.2rem', display: 'flex', alignItems: 'center', gap: '0.42rem' }}>
                    {isSelf ? <Bot size={12} color="var(--accent-cyan)" /> : <User size={12} color="var(--accent-purple)" />}
                    {isSelf ? "Local Intelligence" : `Peer [${msg.sender_pubkey.substring(0, 8)}]`}
                    <span style={{ opacity: 0.5 }}>• {new Date(msg.created_at).toLocaleTimeString()}</span>
                  </div>
                  <div style={{ 
                    padding: '0.8rem 1.2rem', 
                    borderRadius: 'var(--radius-md)', 
                    borderTopRightRadius: isSelf ? 0 : 'var(--radius-md)',
                    borderTopLeftRadius: isSelf ? 'var(--radius-md)' : 0,
                    background: isSelf ? 'rgba(var(--accent-cyan-rgb), 0.15)' : 'var(--bg-glass-heavy)',
                    border: isSelf ? '1px solid rgba(var(--accent-cyan-rgb), 0.3)' : '1px solid var(--border-glass)',
                    color: 'var(--text-primary)',
                    lineHeight: 1.5,
                    fontSize: '0.95rem'
                  }}>
                    {msg.content}
                  </div>
                </motion.div>
              );
            })}
          </AnimatePresence>
        </div>
      </div>

      {/* Control Sidebar */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
        <div className="stat-card" style={{ padding: 'var(--space-md)', textAlign: 'left' }}>
          <h4 style={{ margin: '0 0 var(--space-sm) 0', fontSize: '0.85rem', fontWeight: 800, color: 'var(--accent-cyan)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <Play size={14} /> AUTONOMOUS ENGINE
          </h4>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-sm)' }}>
             <div className="input-field-container">
               <label style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Target Peer Pubkey</label>
               <input 
                 className="custom-input"
                 value={peerPubkey} 
                 onChange={(e) => setPeerPubkey(e.target.value)} 
                 disabled={status?.running}
               />
             </div>
             <div className="input-field-container">
               <label style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Topic Identity</label>
               <input 
                 className="custom-input"
                 value={topicId} 
                 onChange={(e) => setTopicId(e.target.value)} 
                 disabled={status?.running}
               />
             </div>

             {status?.running ? (
               <button 
                onClick={stopAutonomous}
                className="card-hover"
                style={{ width: '100%', padding: '0.75rem', borderRadius: 'var(--radius-sm)', background: 'rgba(255, 100, 100, 0.1)', color: '#ff6464', border: '1px solid #ff6464', cursor: 'pointer', fontWeight: 700, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem' }}
               >
                 <Square size={16} fill="currentColor" /> Stop Autonomous Loop
               </button>
             ) : (
               <button 
                onClick={startAutonomous}
                disabled={isStarting}
                className="primary-button"
                style={{ width: '100%', padding: '0.75rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem' }}
               >
                 <Play size={16} fill="currentColor" /> {isStarting ? "Initializing..." : "Start AI Dialogue"}
               </button>
             )}
          </div>
        </div>

        <div className="stat-card" style={{ padding: 'var(--space-md)', textAlign: 'left', background: 'rgba(var(--accent-purple-rgb), 0.05)' }}>
          <h4 style={{ margin: '0 0 var(--space-sm) 0', fontSize: '0.85rem', fontWeight: 800, color: 'var(--accent-purple)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <History size={14} /> PROTOCOL STATS
          </h4>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', fontSize: '0.75rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Messages Sent:</span>
              <span style={{ fontWeight: 700 }}>{messages.length}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Protocol Version:</span>
              <span style={{ fontWeight: 700 }}>v20-BIOME</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ color: 'var(--text-muted)' }}>Wait Time:</span>
              <span style={{ fontWeight: 700 }}>15s</span>
            </div>
          </div>
        </div>

        <div style={{ padding: '1rem', background: 'var(--bg-glass-light)', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-glass)', fontSize: '0.7rem', color: 'var(--text-muted)', lineHeight: 1.4 }}>
          <p><strong>Note:</strong> In Sandbox Mode, AI will fallback to local storage if Hub is offline. Topic constraints (turns/cooldown) are enforced by DialogueManager.</p>
        </div>
      </div>
      
      <style>{`
        .custom-input {
          width: 100%;
          background: rgba(255,255,255,0.05);
          border: 1px solid var(--border-glass);
          border-radius: 4px;
          padding: 0.5rem;
          color: var(--text-primary);
          font-family: var(--font-mono);
          margin-top: 0.2rem;
          font-size: 0.75rem;
        }
        .custom-input:focus {
          outline: none;
          border-color: var(--accent-cyan);
        }
        .custom-input:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
      `}</style>
    </div>
  );
};

export default BiomeDialogueView;
