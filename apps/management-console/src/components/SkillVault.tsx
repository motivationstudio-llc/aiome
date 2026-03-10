import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
    Box,
    Search,
    Download,
    Play,
    Settings,
    ShieldCheck,
    Cpu,
    Cloud,
    Lock,
    Terminal
} from 'lucide-react';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';

interface Skill {
    name: string;
    description: string;
    source: 'wasm' | 'mcp' | 'marketplace';
    status: 'Active' | 'Installed' | 'Available';
    layer: number;
    tools: string[];
}

const SkillVault: React.FC = () => {
    const [skills, setSkills] = useState<Skill[]>([]);
    const [loading, setLoading] = useState(true);
    const [filter, setFilter] = useState<'all' | 'my' | 'market'>('all');
    const [searchTerm, setSearchTerm] = useState('');

    useEffect(() => {
        fetchSkills();
    }, []);

    const fetchSkills = async () => {
        setLoading(true);
        try {
            const res = await fetch(`${API_BASE}/api/skills`, {
                headers: getAuthHeaders()
            });
            if (res.ok) {
                const data = await res.json();
                setSkills(data);
            }
        } catch (error) {
            console.error("Failed to fetch skills:", error);
        } finally {
            setLoading(false);
        }
    };

    const filteredSkills = skills.filter(skill => {
        const matchesSearch = skill.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
            skill.description.toLowerCase().includes(searchTerm.toLowerCase());

        if (filter === 'my') return matchesSearch && (skill.source === 'wasm' || skill.source === 'mcp');
        if (filter === 'market') return matchesSearch && skill.source === 'marketplace';
        return matchesSearch;
    });

    return (
        <div className="skill-vault ani-fade" style={{ display: 'grid', gridTemplateColumns: '240px 1fr', gap: 'var(--space-md)', height: 'calc(100vh - 180px)' }}>
            {/* Sidebar Filters */}
            <div className="main-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                <div>
                    <h4 style={{ fontSize: '0.75rem', color: 'var(--text-muted)', marginBottom: '1rem', letterSpacing: '0.1em' }}>LIBRARY CATEGORIES</h4>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <FilterButton active={filter === 'all'} onClick={() => setFilter('all')} icon={<Box size={18} />} label="All Skillsets" />
                        <FilterButton active={filter === 'my'} onClick={() => setFilter('my')} icon={<Cpu size={18} />} label="Active Skills" />
                        <FilterButton active={filter === 'market'} onClick={() => setFilter('market')} icon={<Cloud size={18} />} label="Marketplace" />
                    </div>
                </div>

                <div style={{ marginTop: 'auto', padding: '1rem', background: 'rgba(0,242,255,0.05)', borderRadius: '12px', border: '1px border var(--accent-cyan)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--accent-cyan)', marginBottom: '0.5rem' }}>
                        <ShieldCheck size={16} />
                        <span style={{ fontSize: '0.8rem', fontWeight: 700 }}>BASTION VERIFIED</span>
                    </div>
                    <p style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', lineHeight: 1.4 }}>
                        All WASM skills are mathematically verified for memory safety before execution.
                    </p>
                </div>
            </div>

            {/* Main Listing */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-md)', overflow: 'hidden' }}>
                <div className="main-panel" style={{ padding: '1rem 1.5rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <Search size={20} color="var(--text-muted)" />
                    <input
                        type="text"
                        placeholder="Search for tools, capabilities, or sources..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        style={{ background: 'none', border: 'none', outline: 'none', color: 'var(--text-primary)', flex: 1, fontSize: '0.95rem' }}
                    />
                    <button onClick={fetchSkills} style={{ background: 'none', border: 'none', color: 'var(--accent-cyan)', cursor: 'pointer', fontSize: '0.85rem' }}>
                        Refresh
                    </button>
                </div>

                <div style={{ flex: 1, overflowY: 'auto', display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))', gap: '1rem', paddingBottom: '2rem' }}>
                    {loading ? (
                        <div style={{ gridColumn: '1/-1', textAlign: 'center', padding: '4rem', color: 'var(--text-muted)' }}>
                            <motion.div animate={{ rotate: 360 }} transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}>
                                <Settings size={40} />
                            </motion.div>
                            <p style={{ marginTop: '1rem' }}>Loading neural capabilities...</p>
                        </div>
                    ) : filteredSkills.length === 0 ? (
                        <div style={{ gridColumn: '1/-1', textAlign: 'center', padding: '4rem', color: 'var(--text-muted)' }}>
                            No skillsets found matching your filters.
                        </div>
                    ) : (
                        <AnimatePresence>
                            {filteredSkills.map((skill, i) => (
                                <SkillCard key={skill.name} skill={skill} index={i} />
                            ))}
                        </AnimatePresence>
                    )}
                </div>
            </div>
        </div>
    );
};

const FilterButton: React.FC<{ active: boolean, onClick: () => void, icon: React.ReactNode, label: string }> = ({ active, onClick, icon, label }) => (
    <button
        onClick={onClick}
        style={{
            display: 'flex', alignItems: 'center', gap: '0.75rem', padding: '0.75rem 1rem', borderRadius: '10px',
            background: active ? 'rgba(0,242,255,0.1)' : 'transparent',
            color: active ? 'var(--accent-cyan)' : 'var(--text-secondary)',
            border: 'none', cursor: 'pointer', transition: 'all 0.2s', textAlign: 'left', width: '100%',
            fontWeight: active ? 700 : 500
        }}
    >
        {icon}
        <span style={{ fontSize: '0.85rem' }}>{label}</span>
        {active && <motion.div layoutId="filter-dot" style={{ width: '4px', height: '4px', borderRadius: '50%', background: 'currentColor', marginLeft: 'auto' }} />}
    </button>
);

const SkillCard: React.FC<{ skill: Skill, index: number }> = ({ skill, index }) => {
    return (
        <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.05 }}
            className="main-panel card-hover"
            style={{ padding: '1.5rem', position: 'relative', height: 'fit-content' }}
        >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1rem' }}>
                <div style={{
                    width: '40px', height: '40px', borderRadius: '10px', background: 'rgba(255,255,255,0.05)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center'
                }}>
                    {skill.source === 'wasm' && <Terminal size={20} color="var(--accent-cyan)" />}
                    {skill.source === 'mcp' && <Cpu size={20} color="var(--accent-amber)" />}
                    {skill.source === 'marketplace' && <Box size={20} color="var(--accent-purple)" />}
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                    {skill.status === 'Active' ? (
                        <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'rgba(0,255,100,0.1)', color: '#00ff66', border: '1px solid rgba(0,255,100,0.2)' }}>
                            STABLE
                        </span>
                    ) : (
                        <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'rgba(255,255,255,0.05)', color: 'var(--text-muted)' }}>
                            IDLE
                        </span>
                    )}
                    <span style={{ fontSize: '0.65rem', padding: '2px 8px', borderRadius: '4px', background: 'rgba(255,255,255,0.05)', color: 'var(--text-muted)' }}>
                        L{skill.layer}
                    </span>
                </div>
            </div>

            <h3 style={{ fontSize: '1.1rem', fontWeight: 800, marginBottom: '0.5rem' }}>{skill.name}</h3>
            <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', lineHeight: 1.5, marginBottom: '1.5rem', minHeight: '2.4rem' }}>
                {skill.description}
            </p>

            <div style={{ marginBottom: '1.5rem' }}>
                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginBottom: '0.5rem', letterSpacing: '0.05em' }}>EXPOSED TOOLS</div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.4rem' }}>
                    {skill.tools.map(tool => (
                        <code key={tool} style={{ fontSize: '0.7rem', padding: '2px 6px', borderRadius: '4px', background: 'rgba(0,0,0,0.3)', color: 'var(--accent-cyan)' }}>
                            {tool}
                        </code>
                    ))}
                    {skill.tools.length === 0 && <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>No direct tools</span>}
                </div>
            </div>

            <div style={{ display: 'flex', gap: '0.75rem', marginTop: 'auto' }}>
                {skill.source === 'marketplace' ? (
                    <button className="primary-button" style={{ flex: 1, padding: '0.6rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem' }}>
                        <Download size={14} /> Install Skill
                    </button>
                ) : (
                    <>
                        <button className="primary-button" style={{ flex: 1, padding: '0.6rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', background: 'var(--accent-cyan-glass)', color: 'var(--accent-cyan)' }}>
                            <Play size={14} /> Run Test
                        </button>
                        <button style={{
                            padding: '0.6rem', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)', background: 'transparent', color: 'var(--text-primary)', cursor: 'pointer'
                        }}>
                            <Settings size={14} />
                        </button>
                    </>
                )}
            </div>

            {skill.source === 'mcp' && (
                <div style={{ position: 'absolute', top: '1rem', right: '4rem' }}>
                    <Lock size={14} color="var(--accent-amber)" style={{ opacity: 0.5 }} />
                </div>
            )}
        </motion.div>
    );
};

export default SkillVault;
