import { useState, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Shield,
  Dna,
  GitMerge,
  MessageSquare,
  Zap,
  BrainCircuit,
  RefreshCw,
  Terminal
} from "lucide-react";
import { Network } from "vis-network";
import { DataSet } from "vis-data";
import "./App.css";

const API_BASE = "http://localhost:3015";

function App() {
  const [activeTab, setActiveTab] = useState("dashboard");
  const [stats, setStats] = useState({ level: 1, exp: 0, resonance: 0, creativity: 0 });
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/health`);
        if (res.ok) {
          setIsConnected(true);
          const data = await res.json();
          // Mocking level from health check for now or fetch real stats
          setStats(prev => ({ ...prev, level: data.level || 1, exp: data.exp || 0 }));
        }

        // For now, let's try getting karma to see if it works
        const karmaRes = await fetch(`${API_BASE}/api/synergy/karma`);
        if (karmaRes.ok) {
          // We'll update stats once a dedicated endpoint exists or calculate from karma
        }
      } catch (e) {
        setIsConnected(false);
      }
    };

    fetchStatus();
    const timer = setInterval(fetchStatus, 5000);
    return () => clearInterval(timer);
  }, []);

  return (
    <div className="app-container">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="brand">
          <BrainCircuit size={28} color="#00f2ff" />
          <span>Aiome</span>
        </div>

        <nav className="nav-group">
          <h4>Synergy Hub</h4>
          <NavItem
            icon={<Activity size={20} />}
            label="Dashboard"
            active={activeTab === "dashboard"}
            onClick={() => setActiveTab("dashboard")}
          />
          <NavItem
            icon={<Dna size={20} />}
            label="Karma Stream"
            active={activeTab === "karma"}
            onClick={() => setActiveTab("karma")}
          />
          <NavItem
            icon={<GitMerge size={20} />}
            label="Resonance Map"
            active={activeTab === "graph"}
            onClick={() => setActiveTab("graph")}
          />
        </nav>

        <nav className="nav-group">
          <h4>Control</h4>
          <NavItem
            icon={<Shield size={20} />}
            label="Immune System"
            active={activeTab === "immune"}
            onClick={() => setActiveTab("immune")}
          />
          <NavItem
            icon={<MessageSquare size={20} />}
            label="Agent Console"
            active={activeTab === "agent"}
            onClick={() => setActiveTab("agent")}
          />
        </nav>

        <div style={{ marginTop: 'auto', padding: '1rem', background: 'rgba(255,255,255,0.03)', borderRadius: '12px', fontSize: '0.8rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
            <span style={{ color: 'var(--text-secondary)' }}>Samsara Tier</span>
            <span style={{ color: 'var(--accent-purple)' }}>Level {stats.level}</span>
          </div>
          <div style={{ height: '4px', background: 'rgba(255,255,255,0.1)', borderRadius: '2px', overflow: 'hidden' }}>
            <motion.div
              initial={{ width: 0 }}
              animate={{ width: `${(stats.exp % 1000) / 10}%` }}
              style={{ height: '100%', background: 'var(--accent-purple)' }}
            />
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className="main-content">
        <header className="header">
          <motion.h2
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            key={activeTab}
          >
            {activeTab.charAt(0).toUpperCase() + activeTab.slice(1)}
          </motion.h2>

          <div className={`status-badge ${isConnected ? '' : 'disconnected'}`}>
            <div className={`status-dot ${isConnected ? '' : 'offline'}`} />
            {isConnected ? "Samsara Hub Connected" : "Connection Lost"}
          </div>
        </header>

        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
          >
            {activeTab === "dashboard" && <Dashboard stats={stats} isConnected={isConnected} />}
            {activeTab === "karma" && <KarmaStream />}
            {activeTab === "graph" && <GraphView />}
            {activeTab === "agent" && <AgentConsole />}
          </motion.div>
        </AnimatePresence>
      </main>
    </div>
  );
}

function NavItem({ icon, label, active, onClick }: { icon: any, label: string, active: boolean, onClick: () => void }) {
  return (
    <div className={`nav-item ${active ? 'active' : ''}`} onClick={onClick}>
      {icon}
      <span>{label}</span>
    </div>
  );
}

function Dashboard({ isConnected }: { stats: any, isConnected: boolean }) {
  return (
    <>
      <div className="grid-stats">
        <StatCard label="Resonance Score" value="842" trend="+12.4%" color="var(--accent-cyan)" />
        <StatCard label="Immune Rules" value="24" trend="Active" color="#10b981" />
        <StatCard label="Karma Entities" value="156" trend="+5 today" color="var(--accent-purple)" />
        <StatCard label="Sync Nodes" value="12" trend={isConnected ? "Online" : "Connecting..."} color="#f59e0b" />
      </div>

      <div className="main-panel">
        <div className="panel-header">
          <h3>System Vitality</h3>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button className="nav-item" style={{ padding: '0.25rem 0.75rem', fontSize: '0.8rem' }}><RefreshCw size={14} /> Refresh</button>
          </div>
        </div>
        <div style={{ padding: '2rem', display: 'flex', justifyContent: 'center', alignItems: 'center', height: '300px', color: 'var(--text-secondary)' }}>
          <div style={{ textAlign: 'center' }}>
            <Activity size={48} color="var(--accent-cyan)" style={{ marginBottom: '1rem', opacity: 0.5 }} />
            <p>Real-time neural activity metrics will appear here.</p>
          </div>
        </div>
      </div>
    </>
  );
}

function StatCard({ label, value, trend, color }: any) {
  return (
    <div className="stat-card">
      <div className="stat-label">{label}</div>
      <div className="stat-value" style={{ color: color }}>{value}</div>
      <div className="stat-trend trend-up">{trend}</div>
    </div>
  );
}

function KarmaStream() {
  const [karmas, setKarmas] = useState<any[]>([]);

  useEffect(() => {
    fetch(`${API_BASE}/api/synergy/karma`)
      .then(res => res.json())
      .then(data => setKarmas(data))
      .catch(console.error);
  }, []);

  return (
    <div className="main-panel">
      <div className="panel-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <Zap size={20} color="var(--accent-cyan)" />
          <h3>Eternal Karma Log</h3>
        </div>
      </div>
      <div style={{ padding: '1rem', maxHeight: '60vh', overflowY: 'auto' }}>
        {karmas.length === 0 ? (
          <p style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: '3rem' }}>No karma recorded in this aeon.</p>
        ) : (
          karmas.map((k, i) => (
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.05 }}
              key={k.id || i}
              style={{
                padding: '1.25rem',
                borderBottom: '1px solid var(--border-glass)',
                background: 'rgba(255,255,255,0.01)',
                display: 'flex',
                gap: '1rem',
                alignItems: 'flex-start'
              }}
            >
              <div style={{
                width: '32px',
                height: '32px',
                borderRadius: '8px',
                background: k.karma_type === 'Technical' ? 'rgba(0, 242, 255, 0.1)' : 'rgba(188, 140, 255, 0.1)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: k.karma_type === 'Technical' ? 'var(--accent-cyan)' : 'var(--accent-purple)'
              }}>
                {k.karma_type === 'Technical' ? <Terminal size={16} /> : <BrainCircuit size={16} />}
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.25rem' }}>
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontWeight: 600 }}>{k.karma_type.toUpperCase()} | Job #{k.job_id}</span>
                  <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{k.weight}% Weight</span>
                </div>
                <div style={{ fontSize: '0.95rem', lineHeight: 1.5 }}>{k.lesson}</div>
              </div>
            </motion.div>
          ))
        )}
      </div>
    </div>
  );
}

function GraphView() {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const initGraph = async () => {
      try {
        const response = await fetch(`${API_BASE}/api/synergy/graph`);
        const data = await response.json();

        const nodes = new DataSet(data.nodes.map((n: any) => ({
          id: n.id,
          label: n.label,
          group: n.group,
          font: { color: '#f0f2f5', size: 12, face: 'Outfit' },
          shape: n.group === 'core' ? 'diamond' : 'dot',
          borderWidth: n.group.endsWith('_global') ? 3 : 1,
          size: n.group === 'core' ? 30 : 15
        })));

        const edges = new DataSet(data.edges);

        const options = {
          groups: {
            core: { color: { background: '#00f2ff', border: '#fff' } },
            karma_local: { color: { background: '#bc8cff', border: '#bc8cff' } },
            karma_global: { color: { background: 'rgba(188, 140, 255, 0.2)', border: '#bc8cff' } },
            rule_local: { color: { background: '#ff4d94', border: '#ff4d94' } },
            rule_global: { color: { background: 'rgba(255, 77, 148, 0.2)', border: '#ff4d94' } }
          },
          edges: {
            color: { color: 'rgba(255,255,255,0.1)', highlight: '#00f2ff' },
            smooth: { type: 'continuous' },
            width: 1
          },
          physics: {
            enabled: true,
            barnesHut: { gravitationalConstant: -3000, centralGravity: 0.3, springLength: 120 },
            stabilization: { iterations: 100 }
          },
          interaction: {
            hover: true,
            zoomView: true
          }
        };

        new Network(containerRef.current!, { nodes, edges } as any, options as any);
      } catch (e) {
        console.error("Graph failed", e);
      }
    };

    initGraph();
  }, []);

  return (
    <div className="main-panel" style={{ height: '70vh' }}>
      <div className="panel-header">
        <h3>Synapse Resonance Graph</h3>
      </div>
      <div ref={containerRef} style={{ width: '100%', height: 'calc(100% - 60px)' }} />
    </div>
  );
}

function AgentConsole() {
  const [input, setInput] = useState("");
  const [history, setHistory] = useState<any[]>([]);
  const [isTyping, setIsTyping] = useState(false);
  const chatEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(scrollToBottom, [history]);

  const sendMessage = async () => {
    if (!input.trim()) return;
    const userMsg = { role: "user", content: input };
    setHistory(prev => [...prev, userMsg]);
    setInput("");
    setIsTyping(true);

    try {
      // Stream handling would go here for real implementation
      const res = await fetch(`${API_BASE}/api/agent/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ prompt: input, history: history })
      });
      const data = await res.json();
      setHistory(prev => [...prev, { role: "assistant", content: data.response }]);
    } catch (e) {
      setHistory(prev => [...prev, { role: "assistant", content: "⚠️ Connection error to OpenClaw layer." }]);
    } finally {
      setIsTyping(false);
    }
  };

  return (
    <div className="main-panel" style={{ height: '75vh', display: 'flex', flexDirection: 'column' }}>
      <div className="panel-header">
        <h3>Genesis Console</h3>
        <span style={{ fontSize: '0.8rem', color: '#10b981' }}>Live Link Active</span>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        {history.map((m, i) => (
          <div key={i} style={{
            alignSelf: m.role === 'user' ? 'flex-end' : 'flex-start',
            maxWidth: '80%',
            padding: '1rem',
            borderRadius: m.role === 'user' ? '16px 16px 0 16px' : '0 16px 16px 16px',
            background: m.role === 'user' ? 'rgba(0, 242, 255, 0.1)' : 'rgba(255,255,255,0.03)',
            border: m.role === 'user' ? '1px solid rgba(0, 242, 255, 0.2)' : '1px solid var(--border-glass)',
            fontSize: '0.95rem',
            lineHeight: 1.5
          }}>
            {m.content}
          </div>
        ))}
        {isTyping && <div style={{ color: 'var(--text-secondary)', fontSize: '0.8rem', fontStyle: 'italic' }}>Agent is thinking...</div>}
        <div ref={chatEndRef} />
      </div>

      <div style={{ padding: '1.5rem', borderTop: '1px solid var(--border-glass)', background: 'rgba(0,0,0,0.1)' }}>
        <div style={{ display: 'flex', gap: '1rem' }}>
          <input
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && sendMessage()}
            placeholder="Issue skill sequence command..."
            style={{
              flex: 1,
              background: '#0d1117',
              border: '1px solid var(--border-glass)',
              borderRadius: '12px',
              padding: '0.75rem 1rem',
              color: '#fff',
              outline: 'none'
            }}
          />
          <button
            onClick={sendMessage}
            style={{
              background: 'var(--accent-cyan)',
              color: '#000',
              border: 'none',
              borderRadius: '12px',
              padding: '0 1.5rem',
              fontWeight: 600,
              cursor: 'pointer'
            }}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
