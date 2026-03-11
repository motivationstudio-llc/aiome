import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Shield, AlertTriangle, CheckCircle, Search, Filter } from 'lucide-react';
import { API_BASE } from "../config";

import { ImmuneRule } from "../types";
import { getAuthHeaders } from "../lib/auth";

const ImmuneSystem: React.FC = () => {
    const [rules, setRules] = useState<ImmuneRule[]>([]);
    const [isAdding, setIsAdding] = useState(false);
    const [newRule, setNewRule] = useState({ pattern: '', severity: 50, action: 'BLOCK' });

    const fetchRules = async () => {
        try {
            const res = await fetch(`${API_BASE}/api/synergy/rules`, {
                headers: getAuthHeaders()
            });
            if (res.ok) {
                const data: ImmuneRule[] = await res.json();

                // Map backend severity (0-100) to UI risk
                const mapped = data.map(r => ({
                    ...r,
                    risk: r.severity > 80 ? "CRITICAL" : r.severity > 50 ? "HIGH" : "MEDIUM",
                    active: true // Backend doesn't have active field, assume true
                }));
                setRules(mapped);
            }
        } catch (e) {
            console.error("Failed to fetch immune rules", e);
        }
    };

    useEffect(() => {
        fetchRules();
    }, []);

    const handleAddRule = async () => {
        try {
            const res = await fetch(`${API_BASE}/api/synergy/rules`, {
                method: 'POST',
                headers: {
                    ...getAuthHeaders(),
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    id: '',
                    pattern: newRule.pattern,
                    severity: newRule.severity,
                    action: newRule.action,
                    created_at: '',
                })
            });
            if (res.ok) {
                setIsAdding(false);
                setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to add rule", e);
        }
    };

    const [editingId, setEditingId] = useState<string | null>(null);

    const handleEditRule = (rule: ImmuneRule) => {
        setEditingId(rule.id);
        setNewRule({ pattern: rule.pattern, severity: rule.severity, action: rule.action });
        setIsAdding(true);
    };

    const handleUpdateRule = async () => {
        if (!editingId) return;
        try {
            const res = await fetch(`${API_BASE}/api/synergy/rules`, {
                method: 'PUT',
                headers: {
                    ...getAuthHeaders(),
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    id: editingId,
                    pattern: newRule.pattern,
                    severity: newRule.severity,
                    action: newRule.action,
                    created_at: rules.find(r => r.id === editingId)?.created_at || '',
                })
            });
            if (res.ok) {
                setIsAdding(false);
                setEditingId(null);
                setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to update rule", e);
        }
    };

    const handleDeleteRule = async (id: string) => {
        if (!confirm("Are you sure you want to delete this immune rule?")) return;
        try {
            const res = await fetch(`${API_BASE}/api/synergy/rules/${id}`, {
                method: 'DELETE',
                headers: getAuthHeaders()
            });
            if (res.ok) {
                fetchRules();
            }
        } catch (e) {
            console.error("Failed to delete rule", e);
        }
    };

    return (
        <div className="main-panel ani-fade">
            <div className="panel-header">
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Shield size={20} color="var(--accent-rose)" />
                    <h3>Sentinel Immune System</h3>
                </div>
                <div style={{ display: 'flex', gap: '1rem' }}>
                    <button
                        onClick={() => {
                            setIsAdding(!isAdding);
                            if (isAdding) {
                                setEditingId(null);
                                setNewRule({ pattern: '', severity: 50, action: 'BLOCK' });
                            }
                        }}
                        className="nav-item"
                        style={{ margin: 0, padding: '0 1rem', background: isAdding ? 'var(--accent-rose)' : 'var(--accent-cyan)', color: '#000', fontWeight: 700 }}
                    >
                        {isAdding ? 'CANCEL' : 'FORGE NEW RULE'}
                    </button>
                    <div className="status-badge">
                        <CheckCircle size={14} /> ACTIVE PROTECTIONS: {rules.length}
                    </div>
                </div>
            </div>

            <div style={{ padding: '2rem' }}>
                <AnimatePresence>
                    {isAdding && (
                        <motion.div
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: 'auto', opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            style={{ overflow: 'hidden', marginBottom: '2rem' }}
                        >
                            <div style={{ background: 'rgba(255,255,255,0.03)', border: `1px solid ${editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)'}`, borderRadius: 'var(--radius-lg)', padding: '1.5rem', display: 'flex', flexWrap: 'wrap', gap: '1rem', alignItems: 'flex-end' }}>
                                <div style={{ flex: 2 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>PATTERN (REGEX OR TEXT)</label>
                                    <input
                                        value={newRule.pattern}
                                        onChange={e => setNewRule({ ...newRule, pattern: e.target.value })}
                                        placeholder="e.g. /etc/passwd"
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: '#fff', width: '100%', outline: 'none' }}
                                    />
                                </div>
                                <div style={{ flex: 1 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>SEVERITY (0-100)</label>
                                    <input
                                        type="number"
                                        value={newRule.severity}
                                        onChange={e => setNewRule({ ...newRule, severity: parseInt(e.target.value) })}
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: '#fff', width: '100%', outline: 'none' }}
                                    />
                                </div>
                                <div style={{ flex: 1 }}>
                                    <label style={{ fontSize: '0.7rem', color: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', display: 'block', marginBottom: '0.5rem' }}>ACTION</label>
                                    <select
                                        value={newRule.action}
                                        onChange={e => setNewRule({ ...newRule, action: e.target.value })}
                                        style={{ background: 'rgba(0,0,0,0.3)', border: '1px solid var(--border-glass)', borderRadius: 'var(--radius-md)', padding: '0.75rem', color: '#fff', width: '100%', outline: 'none' }}
                                    >
                                        <option value="BLOCK">BLOCK</option>
                                        <option value="QUARANTINE">QUARANTINE</option>
                                        <option value="WARN">WARN</option>
                                    </select>
                                </div>
                                <button
                                    onClick={editingId ? handleUpdateRule : handleAddRule}
                                    style={{ background: editingId ? 'var(--accent-amber)' : 'var(--accent-cyan)', color: '#000', border: 'none', borderRadius: 'var(--radius-md)', padding: '0.75rem 1.5rem', fontWeight: 700, cursor: 'pointer' }}
                                >
                                    {editingId ? 'UPDATE RULE' : 'ACTIVATE RULE'}
                                </button>
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>

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
                                    border: editingId === rule.id ? '1px solid var(--accent-amber)' : '1px solid var(--border-glass)',
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
                                    <button
                                        onClick={() => handleEditRule(rule)}
                                        style={{
                                            background: 'rgba(255,255,255,0.05)',
                                            border: '1px solid var(--border-glass)',
                                            color: '#fff',
                                            padding: '0.5rem 1rem',
                                            borderRadius: '8px',
                                            fontSize: '0.8rem',
                                            cursor: 'pointer',
                                            fontWeight: 600
                                        }}
                                    >
                                        EDIT
                                    </button>
                                    <button
                                        onClick={() => handleDeleteRule(rule.id)}
                                        style={{
                                            background: 'rgba(255, 77, 148, 0.1)',
                                            border: '1px solid rgba(255, 77, 148, 0.2)',
                                            color: 'var(--accent-rose)',
                                            padding: '0.5rem 1rem',
                                            borderRadius: '8px',
                                            fontSize: '0.8rem',
                                            cursor: 'pointer',
                                            fontWeight: 600
                                        }}
                                    >
                                        DELETE
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
