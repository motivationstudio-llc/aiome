import React from 'react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { useDisplayMode } from '../hooks/useDisplayMode';
import { Monitor, Lock, CreditCard, Database } from 'lucide-react';

const SettingsPage: React.FC = () => {
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const { mode, setMode } = useDisplayMode();

    return (
        <div className="settings-page" style={{ paddingBottom: '5rem' }}>
            <div className="settings-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(350px, 1fr))', gap: '2rem' }}>

                {/* 1. Appearance Section */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Monitor size={24} color="var(--accent-cyan)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Appearance</h3>
                    </div>

                    {/* Character Selection */}
                    <div style={{ marginBottom: '2.5rem' }}>
                        <label style={{ display: 'block', color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '1rem' }}>Avatar Character</label>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                            <div
                                onClick={() => setCharacter('female')}
                                style={{
                                    padding: '1rem',
                                    borderRadius: 'var(--radius-md)',
                                    background: character === 'female' ? 'var(--accent-purple-glass)' : 'rgba(255,255,255,0.03)',
                                    border: `1px solid ${character === 'female' ? 'var(--accent-purple)' : 'var(--border-glass)'}`,
                                    cursor: 'pointer',
                                    transition: 'all 0.2s',
                                    textAlign: 'center'
                                }}
                            >
                                <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♀</div>
                                <div style={{ fontSize: '0.9rem', fontWeight: character === 'female' ? 700 : 400 }}>Female</div>
                            </div>
                            <div
                                onClick={() => setCharacter('male')}
                                style={{
                                    padding: '1rem',
                                    borderRadius: 'var(--radius-md)',
                                    background: character === 'male' ? 'var(--accent-cyan-glass)' : 'rgba(255,255,255,0.03)',
                                    border: `1px solid ${character === 'male' ? 'var(--accent-cyan)' : 'var(--border-glass)'}`,
                                    cursor: 'pointer',
                                    transition: 'all 0.2s',
                                    textAlign: 'center'
                                }}
                            >
                                <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♂</div>
                                <div style={{ fontSize: '0.9rem', fontWeight: character === 'male' ? 700 : 400 }}>Male</div>
                            </div>
                        </div>
                    </div>

                    {/* Proportion Selection (Style) */}
                    <div style={{ marginBottom: '2.5rem' }}>
                        <label style={{ display: 'block', color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '1rem' }}>Avatar Style (Proportions)</label>
                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                            <div
                                onClick={() => setProportion('chibi')}
                                style={{
                                    padding: '0.8rem',
                                    borderRadius: 'var(--radius-md)',
                                    background: proportion === 'chibi' ? 'var(--bg-glass-light)' : 'transparent',
                                    border: `1px solid ${proportion === 'chibi' ? (character === 'male' ? 'var(--accent-cyan)' : 'var(--accent-purple)') : 'var(--border-glass)'}`,
                                    cursor: 'pointer',
                                    textAlign: 'center',
                                    fontSize: '0.8rem',
                                    transition: 'all 0.2s',
                                    color: proportion === 'chibi' ? 'var(--text-bright)' : 'var(--text-muted)'
                                }}
                            >
                                Cute Chibi (SD)
                            </div>
                            <div
                                onClick={() => setProportion('taller')}
                                style={{
                                    padding: '0.8rem',
                                    borderRadius: 'var(--radius-md)',
                                    background: proportion === 'taller' ? 'var(--bg-glass-light)' : 'transparent',
                                    border: `1px solid ${proportion === 'taller' ? (character === 'male' ? 'var(--accent-cyan)' : 'var(--accent-purple)') : 'var(--border-glass)'}`,
                                    cursor: 'pointer',
                                    textAlign: 'center',
                                    fontSize: '0.8rem',
                                    transition: 'all 0.2s',
                                    color: proportion === 'taller' ? 'var(--text-bright)' : 'var(--text-muted)'
                                }}
                            >
                                Modern Taller
                            </div>
                        </div>
                    </div>

                    <div>
                        <label style={{ display: 'block', color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '1rem' }}>Display Mode</label>
                        <div style={{ display: 'flex', gap: '0.5rem', background: 'rgba(255,255,255,0.05)', padding: '4px', borderRadius: '10px' }}>
                            {['vrm', 'lite', 'off'].map((m) => (
                                <button
                                    key={m}
                                    onClick={() => setMode(m as any)}
                                    style={{
                                        flex: 1,
                                        padding: '8px',
                                        border: 'none',
                                        background: mode === m ? 'var(--accent-cyan)' : 'transparent',
                                        color: mode === m ? '#000' : 'var(--text-muted)',
                                        borderRadius: '8px',
                                        cursor: 'pointer',
                                        fontSize: '0.8rem',
                                        textTransform: 'capitalize',
                                        transition: 'all 0.2s'
                                    }}
                                >
                                    {m === 'vrm' ? '🌟 ' : m === 'lite' ? '⚡ ' : '🚫 '}{m}
                                </button>
                            ))}
                        </div>
                    </div>
                </section>

                {/* 2. LLM Configuration (Lock) */}
                <LockedSection title="LLM Configuration" icon={<Database size={24} color="var(--accent-purple)" />} />

                {/* 3. Security & Auth (Lock) */}
                <LockedSection title="Security & API Keys" icon={<Lock size={24} color="var(--accent-rose)" />} />

                {/* 4. Subscription (Lock) */}
                <LockedSection title="Subscription & Plans" icon={<CreditCard size={24} color="var(--accent-emerald)" />} />

            </div>
        </div>
    );
};

const LockedSection: React.FC<{ title: string, icon: React.ReactNode }> = ({ title, icon }) => (
    <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)', opacity: 0.6, position: 'relative', overflow: 'hidden' }}>
        <div style={{ position: 'absolute', top: '1rem', right: '1rem' }}>
            <Lock size={16} color="var(--text-muted)" />
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '1rem' }}>
            {icon}
            <h3 style={{ margin: 0, fontSize: '1.2rem' }}>{title}</h3>
        </div>
        <p style={{ fontSize: '0.8rem', color: 'var(--text-muted)' }}>This feature will be available in the upcoming Pro version update.</p>
        <div style={{ height: '100px', background: 'rgba(255,255,255,0.02)', borderRadius: 'var(--radius-md)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <span style={{ fontSize: '0.7rem', color: 'var(--text-muted)', letterSpacing: '2px' }}>PENDING EVOLUTION</span>
        </div>
    </section>
);

export default SettingsPage;
