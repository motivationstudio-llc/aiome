import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Sparkles, Activity, Dna, BrainCircuit, Zap } from 'lucide-react';
import { useSystemVitality } from '../hooks/useSystemVitality';

interface BiotopeViewProps {
    stats: any;
    isConnected: boolean;
}

const BiotopeView: React.FC<BiotopeViewProps> = ({ stats, isConnected }) => {
    const [recentEvents, setRecentEvents] = useState<any[]>([]);
    const [pulseLevel, setPulseLevel] = useState(0);

    const { lastEvent } = useSystemVitality();

    useEffect(() => {
        if (!lastEvent) return;

        const { type, data } = lastEvent;

        switch (type) {
            case 'level_up':
                addEvent('Level Up!', `System ascended to level ${data.level}.`, 'var(--accent-cyan)', <Sparkles size={16} />);
                setPulseLevel(prev => prev + 20);
                break;
            case 'karma_update':
                addEvent('New Karma', data.lesson, 'var(--accent-purple)', <Dna size={16} />);
                setPulseLevel(prev => prev + 10);
                break;
            case 'inspiration':
                addEvent('Inspiration', data.record_type, 'var(--accent-rose)', <BrainCircuit size={16} />);
                break;
            default:
                break;
        }
    }, [lastEvent]);

    const addEvent = (title: string, desc: string, color: string, icon: any) => {
        const id = Date.now();
        setRecentEvents(prev => [{ id, title, desc, color, icon }, ...prev].slice(0, 5));
        // Reset pulse level gradually
        setTimeout(() => setPulseLevel(prev => Math.max(0, prev - 5)), 2000);
    };

    return (
        <div className="biotope-view" style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 'var(--space-md)' }}>
            {/* Left: Avatar & Main Visualization */}
            <div className="main-panel ani-fade" style={{
                display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                padding: '3rem', position: 'relative', minHeight: '500px',
                background: 'rgba(255,255,255,0.01)', // Almost fully transparent
                backdropFilter: 'none', // Remove blur to see VRM clearly
                border: '1px solid rgba(255,255,255,0.05)'
            }}>
                <div style={{ position: 'absolute', top: '1.5rem', left: '1.5rem', display: 'flex', gap: '1rem', alignItems: 'center' }}>
                    <Activity size={20} color="var(--accent-cyan)" className="ani-breath" />
                    <h3 style={{ fontSize: '1rem', fontWeight: 600, color: 'var(--text-muted)' }}>LIVE SYSTEM VITALITY</h3>
                </div>

                {/* Vitality Pulsing rings background */}
                <div style={{ position: 'absolute', zIndex: 0 }}>
                    {[...Array(3)].map((_, i) => (
                        <motion.div
                            key={i}
                            animate={{
                                scale: [1, 1.2 + (pulseLevel / 100)],
                                opacity: [0.15, 0]
                            }}
                            transition={{ duration: 3 - i * 0.5, repeat: Infinity, ease: "easeOut" }}
                            style={{
                                width: 300 + i * 100,
                                height: 300 + i * 100,
                                borderRadius: '50%',
                                border: '1px solid var(--accent-cyan)',
                                position: 'absolute',
                                top: -(150 + i * 50),
                                left: -(150 + i * 50),
                            }}
                        />
                    ))}
                </div>

                <div style={{ zIndex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '2rem' }}>
                    {/* Background Diorama provides the main visual focus */}
                    <div style={{ height: '220px' }} />

                    <div style={{ textAlign: 'center' }}>
                        <h2 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '0.5rem' }}>
                            Level {stats.level} <span style={{ color: 'var(--accent-purple)', fontSize: '1rem', fontWeight: 600 }}>Ascension {Math.floor(stats.level / 10)}</span>
                        </h2>
                        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
                            <Zap size={14} color="var(--accent-amber)" /> {Math.floor(stats.exp / 10)} Energy Resonance
                        </div>
                    </div>
                </div>

                {/* Ambient background words/terms from Karma floating? (Maybe later) */}
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
