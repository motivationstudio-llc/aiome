import React, { useState, useEffect, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Shield,
  Clock,
  GitMerge,
  MessageSquare,
  BrainCircuit,
  Package,
  Box,
  Settings as SettingsIcon,
  Zap
} from "lucide-react";
const OnboardingModal = React.lazy(() => import("./components/OnboardingModal"));
const SystemBirth = React.lazy(() => import("./components/SystemBirth"));
const BiotopeView = React.lazy(() => import("./components/BiotopeView"));
const Timeline = React.lazy(() => import("./components/Timeline"));
const ImmuneSystem = React.lazy(() => import("./components/ImmuneSystem"));
const AgentConsole = React.lazy(() => import("./components/AgentConsole"));
const SkillVault = React.lazy(() => import("./components/SkillVault"));
const ArtifactVault = React.lazy(() => import("./components/ArtifactVault"));
const GraphView = React.lazy(() => import("./components/GraphView"));
const SettingsPage = React.lazy(() => import("./components/SettingsPage"));
import DioramaView from "./components/diorama/DioramaView";
import { useAvatarState } from "./hooks/useAvatarState";
import { useDisplayMode } from "./hooks/useDisplayMode";
import { AgentStats, VitalityUIEvent, Karma } from "./types";
import { useSystemVitality } from "./hooks/useSystemVitality";

function App() {
  const [activeTab, setActiveTab] = useState("dashboard");
  const [stats, setStats] = useState<AgentStats>({ level: 1, exp: 0, resonance: 0, creativity: 0, fatigue: 0 });
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [showBirth, setShowBirth] = useState(false);
  const [recentEvents, setRecentEvents] = useState<VitalityUIEvent[]>([]);

  const { lastEvent, connectionStatus, toggleConnection } = useSystemVitality();

  const isConnected = connectionStatus === 'connected';

  const avatarState = useAvatarState();
  const { mode } = useDisplayMode();

  useEffect(() => {
    const isFirstVisit = localStorage.getItem("aiome_onboarding_done") !== "true";
    if (isFirstVisit) {
      setShowOnboarding(true);
    }
  }, []);

  // Global event processor & stats updater
  useEffect(() => {
    if (!lastEvent) return;
    const { type, data } = lastEvent;

    const addEvent = (title: string, desc: string, color: string, icon: React.ReactNode) => {
      const id = Date.now();
      setRecentEvents((prev: VitalityUIEvent[]) => [{ id, title, desc, color, icon }, ...prev].slice(0, 5));
    };

    switch (type) {
      case 'level_up': {
        const d = data as AgentStats;
        setStats(prev => ({ ...prev, level: d.level, exp: d.exp }));
        addEvent('Level Up', `Ascension Level ${d.level}`, 'var(--accent-cyan)', <Activity size={16} />);
        break;
      }
      case 'karma_update': {
        const d = data as Karma;
        addEvent('Karma Assimilated', `Synapses merged: ${d.id.substring(0, 8)}`, 'var(--accent-purple)', <GitMerge size={16} />);
        break;
      }
      case 'immune_alert': {
        const d = data as any;
        addEvent('Security Alert', d.description || "Anomaly detected.", 'var(--accent-rose)', <Shield size={16} />);
        break;
      }
      case 'job_started': {
        addEvent('Deliberation Started', typeof data === 'string' ? data : 'Thinking...', 'var(--accent-amber)', <Activity size={16} />);
        break;
      }
      case 'skill_execution': {
        addEvent('Skill Activating', typeof data === 'string' ? data : 'Tool Execution', 'var(--accent-emerald)', <Zap size={16} />);
        break;
      }
      case 'inspiration': {
        const d = data as any;
        addEvent('Inspiration', d.description || "Creative spark detected.", 'var(--accent-rose)', <BrainCircuit size={16} />);
        break;
      }
      case 'agent_stats': {
        const d = data as AgentStats;
        setStats(d);
        break;
      }
      case 'proactive_talk': {
        const d = data as string;
        addEvent('Aiome Message', d, 'var(--accent-cyan)', <MessageSquare size={16} />);
        break;
      }
      default:
        break;
    }
  }, [lastEvent]);

  // Status Badge Rendering Logic
  const renderStatusBadge = () => {
    let badgeClass = "status-badge";
    let dotClass = "status-dot";
    let text = "";

    switch (connectionStatus) {
      case "connected":
        text = "Samsara Hub Connected";
        // Default classes are fine
        break;
      case "connecting":
        badgeClass += ' disconnected'; // Using disconnected style for connecting state
        dotClass += ' offline'; // Using offline dot style for connecting state
        dotClass += ' ani-pulse';
        text = "Reconnecting...";
        break;
      case "paused":
        badgeClass += ' paused';
        dotClass += ' offline';
        dotClass = dotClass.replace('offline', 'paused'); // Custom styling inline if needed
        text = "Sync Paused";
        break;
      case "disconnected":
      default:
        badgeClass += ' disconnected';
        dotClass += ' offline';
        text = "Connection Lost";
        break;
    }

    return (
      <button
        className={badgeClass}
        onClick={toggleConnection}
        style={{
          cursor: 'pointer', border: '1px solid rgba(255,255,255,0.05)', background: 'rgba(0,0,0,0.4)',
          outline: 'none', transition: 'all 0.2s', padding: '0.5rem 1rem'
        }}
        title="Click to toggle connection sync"
      >
        <div className={dotClass} style={{
          background: connectionStatus === 'paused' ? 'var(--accent-amber)' : undefined,
          boxShadow: connectionStatus === 'paused' ? 'var(--glow-amber)' : undefined
        }} />
        {text}
      </button>
    );
  };

  return (
    <div className="app-container">
      {/* Digital Diorama — Resident Avatar */}
      <DioramaView status={avatarState} mode={mode} activeTab={activeTab} />

      {/* Ambient Background Particles */}
      <div style={{ position: 'fixed', inset: 0, pointerEvents: 'none', zIndex: 0, overflow: 'hidden' }}>
        {useMemo(() => [...Array(6)].map((_, i) => (
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
        )), [])}
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
          <NavItem
            icon={<Box size={20} />}
            label="Artifact Vault"
            active={activeTab === "artifacts"}
            onClick={() => setActiveTab("artifacts")}
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
          <NavItem
            icon={<Package size={20} />}
            label="Skill Vault"
            active={activeTab === "vault"}
            onClick={() => setActiveTab("vault")}
          />
          <NavItem
            icon={<SettingsIcon size={20} />}
            label="Settings"
            active={activeTab === "settings"}
            onClick={() => setActiveTab("settings")}
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
            AIOME v1.0.2
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
            {activeTab === "vault" && "Neural Skill Vault"}
            {activeTab === "artifacts" && "Artifact Vault"}
            {activeTab === "settings" && "System Settings"}
          </motion.h2>

          <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
            {renderStatusBadge()}
          </div>
        </header>

        <AnimatePresence mode="wait">
          {/* Use Suspense for lazy loaded components */}
          <React.Suspense fallback={<div style={{ height: '70vh', display: 'flex', alignItems: 'center', justifyContent: 'center' }}><div className="ani-pulse" style={{ color: 'var(--accent-cyan)', fontWeight: 700 }}>NEURAL SYNC...</div></div>}>
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
            >
              {activeTab === "dashboard" && <BiotopeView stats={stats} isConnected={isConnected} recentEvents={recentEvents} />}
              {activeTab === "karma" && <Timeline />}
              {activeTab === "graph" && <GraphView />}
              {activeTab === "immune" && <ImmuneSystem />}
              {activeTab === "agent" && <AgentConsole />}
              {activeTab === "vault" && <SkillVault />}
              {activeTab === "artifacts" && <ArtifactVault />}
              {activeTab === "settings" && <SettingsPage />}
            </motion.div>
          </React.Suspense>
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

function NavItem({ icon, label, active, onClick }: { icon: React.ReactNode, label: string, active: boolean, onClick: () => void }) {
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
