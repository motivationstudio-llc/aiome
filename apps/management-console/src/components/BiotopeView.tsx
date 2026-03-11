import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Activity, Zap } from 'lucide-react';
import { AgentStats, VitalityUIEvent } from '../types';

interface BiotopeViewProps {
    stats: AgentStats;
    isConnected: boolean;
    recentEvents: VitalityUIEvent[];
}

const BiotopeView: React.FC<BiotopeViewProps> = ({ stats, isConnected, recentEvents }) => {
    const [pulseLevel, setPulseLevel] = useState(0);

    // Local visual pulse effect still responds to stats changes for flair
    useEffect(() => {
        setPulseLevel(prev => Math.min(100, prev + 20));
    }, [stats.level]);

    useEffect(() => {
        if (pulseLevel <= 0) return;
        const timer = setTimeout(() => setPulseLevel(prev => Math.max(0, prev - 5)), 2000);
        return () => clearTimeout(timer);
    }, [pulseLevel]);

    return (
        <div className="biotope-view" style={{ display: 'grid', gridTemplateColumns: '1fr var(--layout-right-panel-width)', gap: 'var(--layout-panel-gap)' }}>
            {/* Left: Avatar & Main Visualization */}
            <div className="main-panel ani-fade" style={{
                display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                padding: '3rem', position: 'relative', minHeight: '500px',
                background: 'rgba(255,255,255,0.01)', // Almost fully transparent
                backdropFilter: 'none', // Remove blur to see VRM clearly
                border: '1px solid rgba(255,255,255,0.05)',
                overflow: 'hidden'
            }}>
                <div style={{ position: 'absolute', top: '1.5rem', left: '1.5rem', display: 'flex', gap: '1rem', alignItems: 'center', zIndex: 10 }}>
                    <Activity size={20} color="var(--accent-cyan)" className="ani-breath" />
                    <h3 style={{ fontSize: '1rem', fontWeight: 600, color: 'var(--text-muted)' }}>LIVE SYSTEM VITALITY</h3>
                </div>

                {/* MoodRing & Pulsing Aura Background */}
                <div style={{ position: 'absolute', zIndex: 0, top: '40%', left: '50%', transform: 'translate(-50%, -50%)', pointerEvents: 'none' }}>
                    {/* Outer Cyan/Purple Ring */}
                    <motion.div
                        animate={{ rotate: 360 }}
                        transition={{ duration: 25, repeat: Infinity, ease: 'linear' }}
                        style={{
                            position: 'absolute', top: -200, left: -200, width: 400, height: 400,
                            borderRadius: '50%',
                            background: 'conic-gradient(from 0deg, transparent 0%, rgba(0, 242, 255, 0.5) 25%, transparent 50%, rgba(188, 140, 255, 0.5) 75%, transparent 100%)',
                            WebkitMaskImage: 'radial-gradient(circle, transparent 60%, black 61%)'
                        }}
                    />
                    {/* Inner Rosa/Cyan Ring */}
                    <motion.div
                        animate={{ rotate: -360 }}
                        transition={{ duration: 18, repeat: Infinity, ease: 'linear' }}
                        style={{
                            position: 'absolute', top: -160, left: -160, width: 320, height: 320,
                            borderRadius: '50%', border: '1px solid rgba(255, 255, 255, 0.05)',
                            background: 'conic-gradient(from 90deg, transparent 0%, rgba(255, 77, 148, 0.4) 30%, transparent 60%, rgba(0, 242, 255, 0.4) 90%, transparent 100%)',
                            WebkitMaskImage: 'radial-gradient(circle, transparent 65%, black 66%)'
                        }}
                    />
                    {/* Dynamic Vitality Pulse */}
                    <motion.div
                        animate={{ scale: [1, 1.1 + (pulseLevel / 50), 1], opacity: [0.3, 0.8, 0.3] }}
                        transition={{ duration: 4, repeat: Infinity, ease: 'easeInOut' }}
                        style={{
                            position: 'absolute', top: -140, left: -140, width: 280, height: 280,
                            borderRadius: '50%',
                            background: 'radial-gradient(circle, rgba(0, 242, 255, 0.15) 0%, transparent 70%)',
                            filter: 'blur(15px)'
                        }}
                    />
                </div>

                <div style={{ zIndex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2rem', marginTop: 'auto', pointerEvents: 'none' }}>
                    {/* Space for the Avatar to sit */}
                    <div style={{ height: '320px' }} />

                    <div style={{ textAlign: 'center', background: 'rgba(5, 7, 10, 0.6)', padding: '1rem 2rem', borderRadius: 'var(--radius-lg)', backdropFilter: 'blur(10px)', border: '1px solid var(--border-glass-bright)' }}>
                        <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '0.8rem', textShadow: '0 0 15px rgba(255,255,255,0.3)' }}>
                            Level {stats.level} <span style={{ color: 'var(--accent-purple)', fontSize: '1rem', fontWeight: 600, textShadow: 'var(--glow-purple)' }}>Ascension {Math.floor(stats.level / 10)}</span>
                        </h2>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem', alignItems: 'center' }}>
                            <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
                                <Zap size={14} color="var(--accent-amber)" /> {Math.floor(stats.exp / 10)} Energy Resonance
                            </div>

                            {/* Dynamic Waveform Meter */}
                            <div style={{ display: 'flex', gap: '4px', alignItems: 'flex-end', height: '20px', justifyContent: 'center' }}>
                                {[...Array(16)].map((_, i) => (
                                    <motion.div
                                        key={i}
                                        animate={{ height: ['20%', `${40 + Math.random() * 60}%`, '20%'] }}
                                        transition={{ duration: 0.4 + Math.random() * 0.4, repeat: Infinity, ease: 'easeInOut', delay: i * 0.05 }}
                                        style={{
                                            width: '4px',
                                            backgroundColor: i < ((stats.exp % 1000) / 1000) * 16 ? 'var(--accent-cyan)' : 'var(--border-glass)',
                                            borderRadius: '2px',
                                            boxShadow: i < ((stats.exp % 1000) / 1000) * 16 ? 'var(--glow-cyan)' : 'none'
                                        }}
                                    />
                                ))}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Right: Recent Events Feed */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)' }}>
                <div className="main-panel ani-slide-right" style={{ padding: '0', flex: 1 }}>
                    <div className="panel-header" style={{ padding: '1rem 1.5rem' }}>
                        <h4 style={{ fontSize: '0.85rem', letterSpacing: '0.1em', fontWeight: 700 }}>CHRONICLE PULSE</h4>
                    </div>
                    <div style={{ overflowY: 'auto', padding: '1rem' }}>
                        <AnimatePresence mode="popLayout">
                            {recentEvents.length === 0 ? (
                                <div key="empty" style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                                    Monitoring neural activity...
                                </div>
                            ) : (
                                recentEvents.map(event => (
                                    <motion.div
                                        key={event.id}
                                        initial={{ x: 20, opacity: 0 }}
                                        animate={{ x: 0, opacity: 1 }}
                                        exit={{ x: -20, opacity: 0 }}
                                        style={{
                                            padding: '1rem',
                                            borderRadius: 'var(--radius-md)',
                                            background: 'rgba(255,255,255,0.02)',
                                            borderLeft: `3px solid ${event.color}`,
                                            marginBottom: '0.75rem',
                                            boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
                                            fontSize: '0.85rem'
                                        }}
                                    >
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.4rem', color: event.color, fontWeight: 700 }}>
                                            {event.icon} {event.title}
                                        </div>
                                        <div style={{ color: 'var(--text-secondary)', lineHeight: 1.4, overflow: 'hidden', textOverflow: 'ellipsis', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
                                            {event.desc}
                                        </div>
                                    </motion.div>
                                ))
                            )}
                        </AnimatePresence>
                    </div>
                </div>

                <div className="stat-card ani-slide-right" style={{ padding: '1.25rem' }}>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '0.5rem' }}>SYNERGY HEARTBEAT</div>
                    <div style={{ fontSize: '1.2rem', fontWeight: 800, color: 'var(--accent-emerald)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        {isConnected ? "STABLE" : "WEAK"}
                        <motion.div
                            animate={{ scale: [1, 1.2, 1], opacity: [0.5, 1, 0.5] }}
                            transition={{ duration: 1, repeat: Infinity }}
                            style={{ width: '8px', height: '8px', borderRadius: '50%', background: 'currentColor' }}
                        />
                    </div>
                </div>
            </div>
        </div>
    );
};

export default BiotopeView;
