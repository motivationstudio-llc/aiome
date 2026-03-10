import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Shield,
  Clock,
  GitMerge,
  MessageSquare,
  BrainCircuit
} from "lucide-react";
import OnboardingModal from "./components/OnboardingModal";
import SystemBirth from "./components/SystemBirth";
import BiotopeView from "./components/BiotopeView";
import Timeline from "./components/Timeline";
import ImmuneSystem from "./components/ImmuneSystem";
import AgentConsole from "./components/AgentConsole";
import GraphView from "./components/GraphView";
import DioramaView from "./components/diorama/DioramaView";
import { useAvatarState } from "./hooks/useAvatarState";
import { useDisplayMode } from "./hooks/useDisplayMode";
import { API_BASE } from "./config";

function App() {
  const [activeTab, setActiveTab] = useState("dashboard");
  const [stats, setStats] = useState({ level: 1, exp: 0, resonance: 0, creativity: 0 });
  const [isConnected, setIsConnected] = useState(false);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [showBirth, setShowBirth] = useState(false);

  const avatarState = useAvatarState();
  const { mode, setMode } = useDisplayMode();

  useEffect(() => {
    const isFirstVisit = localStorage.getItem("aiome_onboarding_done") !== "true";
    if (isFirstVisit) {
      setShowOnboarding(true);
    }

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
        // const karmaRes = await fetch(`${API_BASE}/api/synergy/karma`);
        // if (karmaRes.ok) {
        //   // We'll update stats once a dedicated endpoint exists or calculate from karma
        // }
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
      {/* Digital Diorama — Resident Avatar */}
      <DioramaView status={avatarState} />

      {/* Ambient Background Particles */}
      <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0, overflow: 'hidden' }}>
        {[...Array(6)].map((_, i) => (
          <motion.div
            key={i}
            animate={{
              x: [Math.random() * 100 + '%', Math.random() * 100 + '%'],
              y: [Math.random() * 100 + '%', Math.random() * 100 + '%'],
              opacity: [0.1, 0.3, 0.1],
            }}
            transition={{
              duration: 20 + Math.random() * 20,
              repeat: Infinity,
              ease: "linear"
            }}
            style={{
              position: 'absolute',
              width: 300 + Math.random() * 200,
              height: 300 + Math.random() * 200,
              background: i % 2 === 0 ? 'radial-gradient(circle, rgba(0,242,255,0.05) 0%, transparent 70%)' : 'radial-gradient(circle, rgba(188,140,255,0.05) 0%, transparent 70%)',
              borderRadius: '50%',
              filter: 'blur(50px)'
            }}
          />
        ))}
      </div>

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
            label="Biotope"
            active={activeTab === "dashboard"}
            onClick={() => setActiveTab("dashboard")}
          />
          <NavItem
            icon={<Clock size={20} />}
            label="Chronicle"
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
          <div style={{ marginTop: '0.5rem', textAlign: 'center', fontSize: '0.65rem', color: 'var(--text-muted)' }}>
            OPENCLAW CORE v1.0.2
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
            {activeTab === "dashboard" && "Biotope Overview"}
            {activeTab === "karma" && "Eternal Chronicle"}
            {activeTab === "graph" && "Resonance Map"}
            {activeTab === "immune" && "Immune System"}
            {activeTab === "agent" && "Agent Console"}
          </motion.h2>

          <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
            <div className="display-mode-toggle" style={{ display: 'flex', background: 'rgba(255,255,255,0.05)', padding: '2px', borderRadius: '8px', marginRight: '1rem' }}>
              <button
                onClick={() => setMode('vrm')}
                style={{
                  padding: '4px 8px', fontSize: '0.7rem', border: 'none', background: mode === 'vrm' ? 'var(--accent-cyan)' : 'transparent',
                  color: mode === 'vrm' ? '#000' : 'var(--text-muted)', borderRadius: '6px', cursor: 'pointer', transition: 'all 0.2s'
                }}
              >🌟 VRM</button>
              <button
                onClick={() => setMode('lite')}
                style={{
                  padding: '4px 8px', fontSize: '0.7rem', border: 'none', background: mode === 'lite' ? 'var(--accent-cyan)' : 'transparent',
                  color: mode === 'lite' ? '#000' : 'var(--text-muted)', borderRadius: '6px', cursor: 'pointer', transition: 'all 0.2s'
                }}
              >⚡ Lite</button>
              <button
                onClick={() => setMode('off')}
                style={{
                  padding: '4px 8px', fontSize: '0.7rem', border: 'none', background: mode === 'off' ? 'var(--accent-cyan)' : 'transparent',
                  color: mode === 'off' ? '#000' : 'var(--text-muted)', borderRadius: '6px', cursor: 'pointer', transition: 'all 0.2s'
                }}
              >🚫 Off</button>
            </div>

            <div className={`status-badge ${isConnected ? '' : 'disconnected'}`}>
              <div className={`status-dot ${isConnected ? '' : 'offline'}`} />
              {isConnected ? "Samsara Hub Connected" : "Connection Lost"}
            </div>
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
            {activeTab === "dashboard" && <BiotopeView stats={stats} isConnected={isConnected} />}
            {activeTab === "karma" && <Timeline />}
            {activeTab === "graph" && <GraphView />}
            {activeTab === "immune" && <ImmuneSystem />}
            {activeTab === "agent" && <AgentConsole />}
          </motion.div>
        </AnimatePresence>
      </main>

      <OnboardingModal
        isOpen={showOnboarding}
        onClose={() => {
          setShowOnboarding(false);
          localStorage.setItem("aiome_onboarding_done", "true");
          setShowBirth(true);
        }}
      />

      {showBirth && (
        <SystemBirth onComplete={() => {
          setShowBirth(false);
          localStorage.setItem("aiome_birth_shown", "true");
        }} />
      )}
    </div>
  );
}

function NavItem({ icon, label, active, onClick }: { icon: any, label: string, active: boolean, onClick: () => void }) {
  return (
    <div
      className={`nav-item ${active ? 'active' : ''}`}
      onClick={onClick}
    >
      {icon}
      <span>{label}</span>
      {active && <motion.div layoutId="active-pill" className="nav-active-bar" />}
    </div>
  );
}

export default App;
