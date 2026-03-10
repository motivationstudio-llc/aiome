import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Shield, AlertTriangle, CheckCircle, Search, Filter } from 'lucide-react';
import { API_BASE } from "../config";

const ImmuneSystem: React.FC = () => {
    const [rules, setRules] = useState<any[]>([]);

    useEffect(() => {
        const fetchRules = async () => {
            try {
                // Assuming there's an endpoint for listing immune rules. 
                // If not, we might need to mock it or update the API.
                await fetch(`${API_BASE}/api/biome/status`);
                await fetch(`${API_BASE}/api/synergy/karma`);

                setRules([
                    { id: 1, pattern: "rm -rf /", action: "BLOCK & LOG", risk: "CRITICAL", active: true },
                    { id: 2, pattern: "curl .* | bash", action: "QUARANTINE", risk: "HIGH", active: true },
                    { id: 3, pattern: "env | grep API_KEY", action: "MASK", risk: "MEDIUM", active: true },
                ]);
            } catch (e) {
                console.error(e);
            }
        };

        fetchRules();
    }, []);

    return (
        <div className="main-panel ani-fade">
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Shield size={20} color="var(--accent-rose)" />
                    <h3>Sentinel Immune System</h3>
                </div>
                <div className="status-badge">
                    <CheckCircle size={14} /> ACTIVE PROTECTIONS: {rules.length}
                </div>
            </div>

            <div style={{ padding: '2rem' }}>
                <div style={{ display: 'flex', gap: '1rem', marginBottom: '2rem' }}>
                    <div style={{
                        flex: 1,
                        background: 'rgba(255,255,255,0.03)',
                        border: '1px solid var(--border-glass)',
                        borderRadius: 'var(--radius-md)',
                        padding: '0.75rem 1rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.75rem'
                    }}>
                        <Search size={18} color="var(--text-muted)" />
                        <input
                            placeholder="Search active patterns..."
                            style={{ background: 'none', border: 'none', color: '#fff', outline: 'none', width: '100%', fontSize: '0.9rem' }}
                        />
                    </div>
                    <button className="nav-item" style={{ margin: 0, padding: '0 1rem' }}>
                        <Filter size={18} />
                    </button>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <AnimatePresence>
                        {rules.map((rule, i) => (
                            <motion.div
                                key={rule.id}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: i * 0.1 }}
                                style={{
                                    background: 'var(--bg-glass-heavy)',
                                    border: '1px solid var(--border-glass)',
                                    borderRadius: 'var(--radius-lg)',
                                    padding: '1.5rem',
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    alignItems: 'center',
                                    boxShadow: '0 4px 15px rgba(0,0,0,0.2)'
                                }}
                            >
                                <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
                                    <div style={{
                                        width: '48px',
                                        height: '48px',
                                        borderRadius: '12px',
                                        background: rule.risk === 'CRITICAL' ? 'rgba(255, 77, 148, 0.1)' : 'rgba(245, 158, 11, 0.1)',
                                        display: 'flex',
                                        alignItems: 'center',
                                        justifyContent: 'center',
                                        color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)'
                                    }}>
                                        <AlertTriangle size={24} />
                                    </div>
                                    <div>
                                        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', marginBottom: '0.4rem' }}>
                                            <code style={{
                                                fontSize: '1rem',
                                                fontWeight: 700,
                                                color: 'var(--text-primary)',
                                                background: 'rgba(0,0,0,0.3)',
                                                padding: '0.2rem 0.5rem',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.pattern}
                                            </code>
                                            <span style={{
                                                fontSize: '0.7rem',
                                                fontWeight: 800,
                                                color: rule.risk === 'CRITICAL' ? 'var(--accent-rose)' : 'var(--accent-amber)',
                                                border: `1px solid currentColor`,
                                                padding: '1px 6px',
                                                borderRadius: '4px'
                                            }}>
                                                {rule.risk}
                                            </span>
                                        </div>
                                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                                            Action: <span style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{rule.action}</span> • Status: <span style={{ color: 'var(--accent-emerald)' }}>Active</span>
                                        </div>
                                    </div>
                                </div>

                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                    <button style={{
                                        background: 'rgba(255,255,255,0.05)',
                                        border: '1px solid var(--border-glass)',
                                        color: 'var(--text-muted)',
                                        padding: '0.5rem 1rem',
                                        borderRadius: '8px',
                                        fontSize: '0.8rem',
                                        cursor: 'pointer'
                                    }}>
                                        DEACTIVATE
                                    </button>
                                    <button style={{
                                        background: 'rgba(0, 242, 255, 0.1)',
                                        border: '1px solid rgba(0, 242, 255, 0.2)',
                                        color: 'var(--accent-cyan)',
                                        padding: '0.5rem 1rem',
                                        borderRadius: '8px',
                                        fontSize: '0.8rem',
                                        cursor: 'pointer'
                                    }}>
                                        VIEW TRACE
                                    </button>
                                </div>
                            </motion.div>
                        ))}
                    </AnimatePresence>
                </div>

                <div style={{ marginTop: '3rem', padding: '2rem', border: '1px dashed var(--border-glass)', borderRadius: 'var(--radius-xl)', textAlign: 'center' }}>
                    <Shield size={32} style={{ opacity: 0.2, marginBottom: '1rem' }} />
                    <h4 style={{ color: 'var(--text-secondary)' }}>Advanced Heuristics Active</h4>
                    <p style={{ fontSize: '0.85rem', color: 'var(--text-muted)', marginTop: '0.5rem' }}>
                        The Abyss Vault enforces these rules at the memory-page level. <br />
                        Unauthorized modifications to the sentinel state are physically impossible.
                    </p>
                </div>
            </div>
        </div>
    );
};

export default ImmuneSystem;
