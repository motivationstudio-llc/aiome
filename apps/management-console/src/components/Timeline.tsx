import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Zap, Terminal, BrainCircuit, Globe, Clock } from 'lucide-react';
import { API_BASE } from "../config";

const Timeline: React.FC = () => {
    const [karmas, setKarmas] = useState<any[]>([]);
    const [selfNodeId, setSelfNodeId] = useState<string>("");

    useEffect(() => {
        // Fetch health to get self node id
        fetch(`${API_BASE}/api/health`)
            .then(res => res.json())
            .then(data => setSelfNodeId(data.node_id))
            .catch(console.error);

        fetch(`${API_BASE}/api/synergy/karma`)
            .then(res => res.json())
            .then(data => setKarmas(data))
            .catch(console.error);
    }, []);

    return (
        <div className="main-panel ani-fade">
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Clock size={20} color="var(--accent-cyan)" />
                    <h3>Eternal Chronicle - Karma Timeline</h3>
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                    {karmas.length} ENTRIES RECORDED
                </div>
            </div>

            <div style={{ padding: '1.5rem', maxHeight: '75vh', overflowY: 'auto' }}>
                {karmas.length === 0 ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)' }}>
                        <Zap size={48} style={{ opacity: 0.1, marginBottom: '1rem' }} />
                        <p>No karma recorded in this aeon.</p>
                    </div>
                ) : (
                    <div style={{ position: 'relative' }}>
                        {/* Vertical Timeline Line */}
                        <div style={{
                            position: 'absolute',
                            left: '16px',
                            top: '0',
                            bottom: '0',
                            width: '2px',
                            background: 'linear-gradient(to bottom, var(--accent-cyan), var(--accent-purple), transparent)',
                            opacity: 0.2
                        }} />

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                            {karmas.map((k, i) => {
                                const isLocal = k.node_id === selfNodeId || !k.node_id;
                                return (
                                    <motion.div
                                        initial={{ opacity: 0, x: -20 }}
                                        animate={{ opacity: 1, x: 0 }}
                                        transition={{ delay: i * 0.05 }}
                                        key={k.id || i}
                                        style={{
                                            display: 'flex',
                                            gap: '1.5rem',
                                            paddingLeft: '0.5rem'
                                        }}
                                    >
                                        <div style={{
                                            width: '24px',
                                            height: '24px',
                                            borderRadius: '50%',
                                            background: isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)',
                                            border: '4px solid var(--bg-dark-obsidian)',
                                            zIndex: 2,
                                            marginTop: '4px',
                                            boxShadow: isLocal ? '0 0 10px rgba(0, 242, 255, 0.4)' : '0 0 10px rgba(188, 140, 255, 0.4)'
                                        }} />

                                        <div style={{
                                            flex: 1,
                                            padding: '1.25rem',
                                            borderRadius: 'var(--radius-lg)',
                                            background: isLocal ? 'rgba(255,255,255,0.03)' : 'rgba(188, 140, 255, 0.05)',
                                            border: '1px solid var(--border-glass)',
                                            position: 'relative',
                                            overflow: 'hidden'
                                        }}>
                                            {/* Background Decoration for Federated Karma */}
                                            {!isLocal && (
                                                <div style={{
                                                    position: 'absolute',
                                                    right: '-20px',
                                                    bottom: '-20px',
                                                    opacity: 0.05,
                                                    transform: 'rotate(-15deg)'
                                                }}>
                                                    <Globe size={120} />
                                                </div>
                                            )}

                                            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.75rem', alignItems: 'center' }}>
                                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                                    <span style={{
                                                        fontSize: '0.7rem',
                                                        fontWeight: 800,
                                                        padding: '0.2rem 0.5rem',
                                                        borderRadius: '4px',
                                                        background: isLocal ? 'rgba(0, 242, 255, 0.1)' : 'rgba(188, 140, 255, 0.1)',
                                                        color: isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)',
                                                        letterSpacing: '0.1em'
                                                    }}>
                                                        {isLocal ? "LOCAL MEMORY" : "FEDERATED MEMORY"}
                                                    </span>
                                                    <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                        {k.karma_type.toUpperCase()} | JOB #{k.job_id}
                                                    </span>
                                                </div>
                                                <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                    {k.weight}% Weight
                                                </span>
                                            </div>

                                            <div style={{ fontSize: '1.05rem', lineHeight: 1.6, color: 'var(--text-primary)' }}>
                                                {k.lesson}
                                            </div>

                                            <div style={{ marginTop: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                                                {k.karma_type === 'Technical' ? <Terminal size={14} /> : <BrainCircuit size={14} />}
                                                <span>{isLocal ? "Recorded in this instance" : `From Node: ${k.node_id.substring(0, 8)}...`}</span>
                                            </div>
                                        </div>
                                    </motion.div>
                                );
                            })}
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
};

export default Timeline;
