import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Zap, Terminal, BrainCircuit, Clock, Sparkles } from 'lucide-react';
import { API_BASE } from "../config";
import { getAuthHeaders } from '../lib/auth';

const Timeline: React.FC = () => {
    const [events, setEvents] = useState<any[]>([]);
    const [selfNodeId, setSelfNodeId] = useState<string>("");
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const authHeader = getAuthHeaders();

        const fetchData = async () => {
            setLoading(true);
            try {
                // Fetch node id
                const healthRes = await fetch(`${API_BASE}/api/health`, { headers: authHeader });
                const health = await healthRes.json();
                setSelfNodeId(health.node_id);

                // Fetch Karma
                const karmaRes = await fetch(`${API_BASE}/api/synergy/karma`, { headers: authHeader });
                const karmas = await karmaRes.json();

                // Fetch Evolution
                const evoRes = await fetch(`${API_BASE}/api/system/evolution`, { headers: authHeader });
                const evos = await evoRes.json();

                // Merge and sort
                const merged = [
                    ...karmas.map((k: any) => ({ ...k, _type: 'karma' })),
                    ...evos.map((e: any) => ({ ...e, _type: 'evolution' }))
                ].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

                setEvents(merged);
            } catch (e) {
                console.error("Failed to fetch timeline data", e);
            } finally {
                setLoading(false);
            }
        };

        fetchData();
    }, []);

    return (
        <div className="main-panel ani-fade">
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Clock size={20} color="var(--accent-cyan)" />
                    <h3>Eternal Chronicle - Karma Timeline</h3>
                </div>
                <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>
                    {events.length} CHRONICLES
                </div>
            </div>

            <div style={{ padding: '1.5rem', maxHeight: '75vh', overflowY: 'auto' }}>
                {loading ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)' }}>
                        <div className="ani-pulse">Synchronizing Chronicles...</div>
                    </div>
                ) : events.length === 0 ? (
                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-muted)' }}>
                        <Zap size={48} style={{ opacity: 0.1, marginBottom: '1rem' }} />
                        <p>No records found in this aeon.</p>
                    </div>
                ) : (
                    <div style={{ position: 'relative' }}>
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
                            {events.map((e, i) => {
                                const isKarma = e._type === 'karma';
                                const isLocal = isKarma ? (e.node_id === selfNodeId || !e.node_id) : true;

                                return (
                                    <motion.div
                                        initial={{ opacity: 0, x: -20 }}
                                        animate={{ opacity: 1, x: 0 }}
                                        transition={{ delay: i * 0.05 }}
                                        key={e.id || i}
                                        style={{ display: 'flex', gap: '1.5rem', paddingLeft: '0.5rem' }}
                                    >
                                        <div style={{
                                            width: '24px',
                                            height: '24px',
                                            borderRadius: '50%',
                                            background: !isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)'),
                                            border: '4px solid var(--bg-dark-obsidian)',
                                            zIndex: 2,
                                            marginTop: '4px',
                                            boxShadow: !isKarma ? 'var(--glow-amber)' : (isLocal ? 'var(--glow-cyan)' : 'var(--glow-purple)')
                                        }} />

                                        <div style={{
                                            flex: 1,
                                            padding: '1.25rem',
                                            borderRadius: 'var(--radius-lg)',
                                            background: !isKarma ? 'rgba(245, 158, 11, 0.05)' : (isLocal ? 'rgba(255,255,255,0.03)' : 'rgba(188, 140, 255, 0.05)'),
                                            border: '1px solid var(--border-glass)',
                                            position: 'relative',
                                            overflow: 'hidden'
                                        }}>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.75rem', alignItems: 'center' }}>
                                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                                    <span style={{
                                                        fontSize: '0.7rem',
                                                        fontWeight: 800,
                                                        padding: '0.2rem 0.5rem',
                                                        borderRadius: '4px',
                                                        background: !isKarma ? 'rgba(245, 158, 11, 0.1)' : (isLocal ? 'rgba(0, 242, 255, 0.1)' : 'rgba(188, 140, 255, 0.1)'),
                                                        color: !isKarma ? 'var(--accent-amber)' : (isLocal ? 'var(--accent-cyan)' : 'var(--accent-purple)'),
                                                        letterSpacing: '0.1em'
                                                    }}>
                                                        {isKarma ? (isLocal ? "LOCAL MEMORY" : "FEDERATED MEMORY") : "EVOLUTION STEP"}
                                                    </span>
                                                    <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                        {isKarma ? `${e.karma_type.toUpperCase()} | JOB #${e.job_id}` : e.event_type.toUpperCase()}
                                                    </span>
                                                </div>
                                                <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>
                                                    {new Date(e.created_at).toLocaleTimeString()}
                                                </span>
                                            </div>

                                            <div style={{ fontSize: '1.05rem', lineHeight: 1.6, color: 'var(--text-primary)' }}>
                                                {isKarma ? e.lesson : e.description}
                                            </div>

                                            {e.inspiration && (
                                                <div style={{ marginTop: '0.5rem', padding: '0.5rem', background: 'rgba(255,255,255,0.05)', borderRadius: '4px', fontSize: '0.85rem', color: 'var(--accent-cyan)', borderLeft: '2px solid var(--accent-cyan)' }}>
                                                    <Sparkles size={12} style={{ marginRight: '0.5rem', verticalAlign: 'middle' }} />
                                                    {e.inspiration}
                                                </div>
                                            )}

                                            <div style={{ marginTop: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
                                                {isKarma ? (e.karma_type === 'Technical' ? <Terminal size={14} /> : <BrainCircuit size={14} />) : <Zap size={14} />}
                                                <span>{new Date(e.created_at).toLocaleDateString()}</span>
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
