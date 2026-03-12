import React, { useState, useEffect } from 'react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { useDisplayMode } from '../hooks/useDisplayMode';
import {
    Monitor, Lock, Database,
    MessageSquare, Globe, Shield, Check, X, Loader2, Cpu, Plus
} from 'lucide-react';
import { API_BASE } from '../config';
import { getAuthHeaders, setAuthToken, authenticatedFetch } from '../lib/auth';
import { useTokenHealth } from '../hooks/useTokenHealth';

interface SettingEntry {
    key: string;
    value: string;
    category: string;
    is_secret: boolean;
    updated_at: string;
}

const SettingsPage: React.FC = () => {
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const { mode, setMode } = useDisplayMode();
    const [settings, setSettings] = useState<SettingEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState<string | null>(null);
    const [testResults, setTestResults] = useState<Record<string, { success: boolean, message: string, loading: boolean }>>({});

    useEffect(() => {
        fetchSettings();
    }, []);

    const fetchSettings = async () => {
        try {
            const res = await fetch(`${API_BASE}/api/v1/settings`, { headers: getAuthHeaders() });
            if (res.ok) {
                const data = await res.json();
                setSettings(data);
            }
        } catch (error) {
            console.error("Failed to fetch settings", error);
        } finally {
            setLoading(false);
        }
    };

    const updateSetting = async (key: string, value: string, category: string) => {
        setSaving(key);
        try {
            const res = await fetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
                body: JSON.stringify({ key, value, category })
            });
            if (res.ok) {
                setSettings(prev => {
                    if (prev.some(s => s.key === key)) {
                        return prev.map(s => s.key === key ? { ...s, value, updated_at: new Date().toISOString() } : s);
                    } else {
                        return [...prev, { key, value, category, is_secret: false, updated_at: new Date().toISOString() }];
                    }
                });
            }
        } catch (error) {
            console.error("Failed to update setting", error);
        } finally {
            setTimeout(() => setSaving(null), 500);
        }
    };

    const testConnection = async (service: string, url: string, model?: string) => {
        if (!url) {
            setTestResults(prev => ({ ...prev, [service]: { success: false, message: 'URL is required', loading: false } }));
            return;
        }
        setTestResults(prev => ({ ...prev, [service]: { success: false, message: '', loading: true } }));
        try {
            const res = await fetch(`${API_BASE}/api/v1/settings/test`, {
                method: 'POST',
                headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
                body: JSON.stringify({ service, url, model })
            });
            const data = await res.json();
            setTestResults(prev => ({ ...prev, [service]: { success: data.success, message: data.message, loading: false } }));
        } catch (error) {
            setTestResults(prev => ({ ...prev, [service]: { success: false, message: 'Connection failed', loading: false } }));
        }
    };

    const getSetting = (key: string) => settings.find(s => s.key === key)?.value || "";

    if (loading) {
        return (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '50vh' }}>
                <Loader2 className="ani-spin" size={40} color="var(--accent-cyan)" />
            </div>
        );
    }

    return (
        <div className="settings-page" style={{ paddingBottom: '8rem' }}>
            <div className="settings-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))', gap: '2rem' }}>

                {/* 1. Appearance Section */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Monitor size={24} color="var(--accent-cyan)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Appearance</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem' }}>
                        <div>
                            <label style={labelStyle}>AI Name</label>
                            <input
                                type="text"
                                value={getSetting('ai_name')}
                                placeholder="Watchtower"
                                onChange={(e) => updateSetting('ai_name', e.target.value, 'identity')}
                                style={{
                                    width: '100%',
                                    background: 'rgba(255,255,255,0.03)',
                                    border: '1px solid var(--border-glass)',
                                    borderRadius: '8px',
                                    padding: '0.8rem',
                                    color: '#fff',
                                    outline: 'none',
                                    fontSize: '0.9rem'
                                }}
                            />
                        </div>

                        <div>
                            <label style={labelStyle}>Avatar Character</label>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                                <div onClick={() => setCharacter('female')} style={charCardStyle(character === 'female', 'purple')}>
                                    <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♀</div>
                                    <div style={{ fontSize: '0.9rem' }}>Female</div>
                                </div>
                                <div onClick={() => setCharacter('male')} style={charCardStyle(character === 'male', 'cyan')}>
                                    <div style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }}>♂</div>
                                    <div style={{ fontSize: '0.9rem' }}>Male</div>
                                </div>
                            </div>
                        </div>

                        <div>
                            <label style={labelStyle}>Avatar Style</label>
                            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                                <div onClick={() => setProportion('chibi')} style={styleCardStyle(proportion === 'chibi', character)}>
                                    Cute Chibi (SD)
                                </div>
                                <div onClick={() => setProportion('taller')} style={styleCardStyle(proportion === 'taller', character)}>
                                    Modern Taller
                                </div>
                            </div>
                        </div>

                        <div>
                            <label style={labelStyle}>Display Mode</label>
                            <div style={{ display: 'flex', gap: '0.3rem', background: 'rgba(255,255,255,0.05)', padding: '4px', borderRadius: '10px' }}>
                                {['vrm', 'lite', 'off'].map((m) => (
                                    <button
                                        key={m}
                                        onClick={() => setMode(m as any)}
                                        style={modeBtnStyle(mode === m)}
                                    >
                                        {m === 'vrm' ? '🌟 ' : m === 'lite' ? '⚡ ' : '🚫 '}{m}
                                    </button>
                                ))}
                            </div>
                        </div>
                    </div>
                </section>

                {/* 2. LLM Configuration Section */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Database size={24} color="var(--accent-purple)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>LLM Engine</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div>
                            <label style={labelStyle}>LLM Provider</label>
                            <select
                                value={getSetting('llm_provider') || 'ollama'}
                                onChange={(e) => update_setting_handler(e.target.value, 'llm_provider', 'llm')}
                                style={selectStyle}
                            >
                                <option value="ollama">Ollama (Local)</option>
                                <option value="lmstudio">LM Studio (Local)</option>
                                <option value="gemini">Google Gemini (Cloud)</option>
                                <option value="openai">OpenAI (Cloud)</option>
                                <option value="claude">Anthropic Claude (Cloud)</option>
                            </select>
                        </div>

                        {(getSetting('llm_provider') === 'ollama' || !getSetting('llm_provider')) && (
                            <>
                                <SettingInput
                                    label="Ollama API Host"
                                    value={getSetting('ollama_host')}
                                    placeholder="http://localhost:11434"
                                    onBlur={(v) => update_setting_handler(v, 'ollama_host', 'llm')}
                                    saving={saving === 'ollama_host'}
                                />
                                <OllamaModelSelector
                                    value={getSetting('ollama_model')}
                                    onSelect={(v) => update_setting_handler(v, 'ollama_model', 'llm')}
                                    saving={saving === 'ollama_model'}
                                />
                            </>
                        )}

                        {getSetting('llm_provider') === 'lmstudio' && (
                            <>
                                <SettingInput
                                    label="LM Studio Host"
                                    value={getSetting('lm_studio_host')}
                                    placeholder="http://localhost:1234"
                                    onBlur={(v) => update_setting_handler(v, 'lm_studio_host', 'llm')}
                                    saving={saving === 'lm_studio_host'}
                                />
                                <SettingInput
                                    label="Model Name"
                                    value={getSetting('llm_model')}
                                    placeholder="loaded model in LM Studio"
                                    onBlur={(v) => update_setting_handler(v, 'llm_model', 'llm')}
                                    saving={saving === 'llm_model'}
                                />
                            </>
                        )}

                        {getSetting('llm_provider') && getSetting('llm_provider') !== 'ollama' && getSetting('llm_provider') !== 'lmstudio' && (
                            <>
                                <div>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                                        <label style={{ ...labelStyle, marginBottom: 0 }}>API Key</label>
                                        {saving === 'llm_api_key' && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
                                    </div>
                                    <input
                                        type="password"
                                        defaultValue={getSetting('llm_api_key')}
                                        placeholder="Enter your API key"
                                        onBlur={(e) => update_setting_handler(e.target.value, 'llm_api_key', 'llm')}
                                        style={inputStyle}
                                    />
                                    <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: '0.4rem', fontStyle: 'italic' }}>
                                        Masked for security. Priority: .env &gt; Database.
                                    </div>
                                </div>
                                <SettingInput
                                    label="Model Name"
                                    value={getSetting('llm_model')}
                                    placeholder={getSetting('llm_provider') === 'gemini' ? 'gemini-2.0-flash' : getSetting('llm_provider') === 'openai' ? 'gpt-4o' : 'claude-3-5-sonnet-20240620'}
                                    onBlur={(v) => update_setting_handler(v, 'llm_model', 'llm')}
                                    saving={saving === 'llm_model'}
                                />
                            </>
                        )}

                        <div style={{ marginTop: '0.5rem' }}>
                            <button
                                onClick={() => {
                                    const provider = getSetting('llm_provider') || 'ollama';
                                    if (provider === 'ollama') {
                                        testConnection('ollama', getSetting('ollama_host') || 'http://localhost:11434', getSetting('ollama_model') || 'qwen2.5-coder:7b');
                                    } else {
                                        // TODO: Cloud connection test in API server if needed
                                        alert("Cloud provider connection testing is not yet fully implemented in the bridge. Settings saved.");
                                    }
                                }}
                                style={testBtnStyle}
                                disabled={testResults['ollama']?.loading}
                            >
                                {testResults['ollama']?.loading ? <Loader2 className="ani-spin" size={14} /> : <Cpu size={14} />}
                                Test {(getSetting('llm_provider') || 'ollama').toUpperCase()} Connection
                            </button>
                            {testResults['ollama'] && (
                                <div style={testResultStyle(testResults['ollama'].success)}>
                                    {testResults['ollama'].success ? <Check size={12} /> : <X size={12} />}
                                    {testResults['ollama'].message}
                                </div>
                            )}
                        </div>
                    </div>
                </section>

                {/* 2.5 Background LLM (Autonomous) Section */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Cpu size={24} color="var(--accent-fuchsia)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Background LLM (Autonomous)</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div>
                            <label style={labelStyle}>Provider</label>
                            <select
                                value={getSetting('bg_llm_provider') || 'gemini'}
                                onChange={(e) => update_setting_handler(e.target.value, 'bg_llm_provider', 'llm')}
                                style={selectStyle}
                            >
                                <option value="gemini">Google Gemini (Cloud)</option>
                                <option value="openai">OpenAI (Cloud)</option>
                                <option value="claude">Anthropic Claude (Cloud)</option>
                                <option value="lmstudio">LM Studio (Local)</option>
                                <option value="ollama">Ollama (Not Recommended)</option>
                            </select>
                        </div>

                        {(getSetting('bg_llm_provider') === 'gemini' || getSetting('bg_llm_provider') === 'openai' || getSetting('bg_llm_provider') === 'claude') && (
                            <div>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                                    <label style={{ ...labelStyle, marginBottom: 0 }}>API Key</label>
                                    {saving === 'bg_llm_api_key' && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
                                </div>
                                <input
                                    type="password"
                                    defaultValue={getSetting('bg_llm_api_key')}
                                    placeholder="Enter Background API key (or leave empty for .env default)"
                                    onBlur={(e) => update_setting_handler(e.target.value, 'bg_llm_api_key', 'llm')}
                                    style={inputStyle}
                                />
                                <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: '0.4rem', fontStyle: 'italic' }}>
                                    If left empty, falls back to the main LLM API Key or .env API keys.
                                </div>
                            </div>
                        )}

                        <SettingInput
                            label="Model Name"
                            value={getSetting('bg_llm_model')}
                            placeholder={getSetting('bg_llm_provider') === 'gemini' ? 'gemini-2.5-flash' : 'Model name'}
                            onBlur={(v) => update_setting_handler(v, 'bg_llm_model', 'llm')}
                            saving={saving === 'bg_llm_model'}
                        />
                    </div>
                </section>

                {/* 3. Channel Integration Section */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <MessageSquare size={24} color="#5865F2" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Channel Bridges</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <VaultProtectionItem label="Discord Token" />
                        <SettingInput
                            label="Discord Chat Channel ID"
                            value={getSetting('discord_chat_channel_id')}
                            placeholder="1234567890..."
                            onBlur={(v) => update_setting_handler(v, 'discord_chat_channel_id', 'channel')}
                            saving={saving === 'discord_chat_channel_id'}
                        />
                        <div style={{ borderTop: '1px solid rgba(255,255,255,0.05)', margin: '0.5rem 0' }} />
                        <VaultProtectionItem label="Telegram Token" />
                        <SettingInput
                            label="Telegram Chat ID"
                            value={getSetting('telegram_chat_id')}
                            placeholder="-1001234567..."
                            onBlur={(v) => update_setting_handler(v, 'telegram_chat_id', 'channel')}
                            saving={saving === 'telegram_chat_id'}
                        />

                        <div style={{ marginTop: '1rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.03)', padding: '1rem', borderRadius: 'var(--radius-md)' }}>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.8rem' }}>
                                <Globe size={18} color="var(--accent-cyan)" />
                                <div>
                                    <div style={{ fontSize: '0.9rem', fontWeight: 600 }}>Enable Watchtower</div>
                                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Background bridge service</div>
                                </div>
                            </div>
                            <button
                                onClick={() => update_setting_handler(getSetting('watchtower_enabled') === 'true' ? 'false' : 'true', 'watchtower_enabled', 'system')}
                                style={toggleBtnStyle(getSetting('watchtower_enabled') === 'true')}
                            >
                                <div style={toggleCircleStyle(getSetting('watchtower_enabled') === 'true')} />
                            </button>
                        </div>
                    </div>
                </section>

                {/* 4. Security & System */}
                <section className="glass-panel" style={{ padding: '2rem', borderRadius: 'var(--radius-lg)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
                        <Shield size={24} color="var(--accent-rose)" />
                        <h3 style={{ margin: 0, fontSize: '1.2rem' }}>Security & System</h3>
                    </div>

                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <VaultProtectionItem label="API Server Secret" />
                        <SecretUpdater />

                        {/* Allowed Origins (Dynamic CORS) */}
                        <OriginsManager
                            origins={getSetting('allowed_origins')}
                            onSave={(val: string) => update_setting_handler(val, 'allowed_origins', 'cors')}
                            saving={saving === 'allowed_origins'}
                        />

                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <div style={{ fontSize: '0.9rem' }}>Enforce Guardrails</div>
                            <button
                                onClick={() => update_setting_handler(getSetting('enforce_guardrail') === 'true' ? 'false' : 'true', 'enforce_guardrail', 'security')}
                                style={toggleBtnStyle(getSetting('enforce_guardrail') === 'true')}
                            >
                                <div style={toggleCircleStyle(getSetting('enforce_guardrail') === 'true')} />
                            </button>
                        </div>

                        <div>
                            <label style={labelStyle}>Log Level</label>
                            <select
                                value={getSetting('log_level')}
                                onChange={(e) => update_setting_handler(e.target.value, 'log_level', 'system')}
                                style={selectStyle}
                            >
                                {['trace', 'debug', 'info', 'warn', 'error'].map(l => (
                                    <option key={l} value={l}>{l.toUpperCase()}</option>
                                ))}
                            </select>
                        </div>
                    </div>
                </section>

            </div>
        </div>
    );

    function update_setting_handler(val: string, key: string, cat: string) {
        if (getSetting(key) !== val) {
            updateSetting(key, val, cat);
        }
    }
};

// --- Subcomponents ---

const VaultProtectionItem: React.FC<{ label: string }> = ({ label }) => (
    <div>
        <label style={labelStyle}>{label}</label>
        <div style={{
            display: 'flex', alignItems: 'center', gap: '0.8rem',
            background: 'rgba(0,0,0,0.2)', padding: '0.8rem', borderRadius: 'var(--radius-sm)',
            border: '1px solid var(--border-glass)'
        }}>
            <Lock size={14} color="var(--text-muted)" />
            <div style={{ fontSize: '0.8rem', color: 'var(--text-muted)', flex: 1 }}>••••••••••••••••</div>
            <div style={{
                fontSize: '0.6rem', background: 'var(--accent-cyan-glass)',
                color: 'var(--accent-cyan)', padding: '2px 6px', borderRadius: '4px',
                border: '1px solid rgba(0,242,255,0.2)', fontWeight: 700, letterSpacing: '0.5px'
            }}>
                VAULT PROTECTED
            </div>
        </div>
        <div style={{ fontSize: '0.65rem', color: 'var(--text-muted)', marginTop: '0.4rem', fontStyle: 'italic' }}>
            Managed via .env / Abyss Vault for maximum security.
        </div>
    </div>
);

const OriginsManager: React.FC<{ origins: string, onSave: (val: string) => void, saving?: boolean }> = ({ origins, onSave, saving }) => {
    const [items, setItems] = useState<string[]>([]);
    const [draft, setDraft] = useState('');
    const [error, setError] = useState('');

    useEffect(() => {
        setItems(origins ? origins.split(',').map(s => s.trim()).filter(Boolean) : []);
    }, [origins]);

    const isValidOrigin = (v: string) => /^https?:\/\/[^\s,]+$/.test(v);

    const addOrigin = () => {
        const val = draft.trim();
        if (!val) return;
        if (!isValidOrigin(val)) { setError('Invalid URL format (must start with http:// or https://)'); return; }
        if (items.includes(val)) { setError('Origin already exists'); return; }
        const next = [...items, val];
        setItems(next);
        setDraft('');
        setError('');
        onSave(next.join(','));
    };

    const removeOrigin = (idx: number) => {
        const next = items.filter((_, i) => i !== idx);
        setItems(next);
        onSave(next.join(','));
    };

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>Allowed Origins (CORS)</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.4rem', marginBottom: '0.6rem' }}>
                {items.map((item, i) => (
                    <div key={i} style={{
                        display: 'flex', alignItems: 'center', gap: '0.4rem',
                        background: 'rgba(0,242,255,0.08)', border: '1px solid rgba(0,242,255,0.2)',
                        borderRadius: '6px', padding: '0.3rem 0.6rem', fontSize: '0.75rem',
                        color: 'var(--accent-cyan)'
                    }}>
                        <span>{item}</span>
                        <X size={12} style={{ cursor: 'pointer', opacity: 0.6 }} onClick={() => removeOrigin(i)} />
                    </div>
                ))}
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                    type="text" value={draft} placeholder="https://example.com"
                    onChange={(e) => { setDraft(e.target.value); setError(''); }}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') addOrigin(); }}
                    style={{ ...inputStyle, flex: 1 }}
                />
                <button onClick={addOrigin} style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    <Plus size={14} /> Add
                </button>
            </div>
            {error && <div style={{ fontSize: '0.7rem', color: 'var(--accent-rose)', marginTop: '0.4rem' }}>{error}</div>}
            <div style={{ fontSize: '0.6rem', color: 'var(--text-muted)', marginTop: '0.4rem', fontStyle: 'italic' }}>
                ⚠️ Server restart required after changes.
            </div>
        </div>
    );
};

const SecretUpdater: React.FC = () => {
    const [newSecret, setNewSecret] = useState('');
    const [result, setResult] = useState<{ success: boolean, message: string } | null>(null);
    const [testing, setTesting] = useState(false);
    const { isExpired, dismiss } = useTokenHealth();

    const handleUpdate = async () => {
        if (!newSecret.trim()) return;
        setTesting(true);
        setAuthToken(newSecret.trim());
        try {
            const res = await fetch(`${API_BASE}/api/health`, {
                headers: { 'Authorization': `Bearer ${newSecret.trim()}` },
            });
            if (res.ok) {
                setResult({ success: true, message: 'Connection verified! Token saved.' });
                setNewSecret('');
                dismiss();
            } else {
                setResult({ success: false, message: `Authentication failed (${res.status})` });
            }
        } catch {
            setResult({ success: false, message: 'Connection failed' });
        } finally {
            setTesting(false);
        }
    };

    return (
        <div>
            {isExpired && (
                <div style={{
                    background: 'rgba(255,77,148,0.08)', border: '1px solid rgba(255,77,148,0.3)',
                    borderRadius: '8px', padding: '0.8rem', marginBottom: '0.8rem',
                    fontSize: '0.8rem', color: 'var(--accent-rose)',
                    display: 'flex', alignItems: 'center', gap: '0.5rem'
                }}>
                    <Shield size={16} /> Token expired or changed on server. Please update below.
                </div>
            )}
            <label style={labelStyle}>Update API Secret</label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <input
                    type="password" value={newSecret} placeholder="Enter new API secret"
                    onChange={(e) => { setNewSecret(e.target.value); setResult(null); }}
                    onKeyDown={(e) => { if (e.nativeEvent.isComposing) return; if (e.key === 'Enter') handleUpdate(); }}
                    style={{ ...inputStyle, flex: 1 }}
                />
                <button onClick={handleUpdate} disabled={testing} style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    {testing ? <Loader2 size={14} className="ani-spin" /> : <Check size={14} />}
                    Verify
                </button>
            </div>
            {result && (
                <div style={testResultStyle(result.success)}>
                    {result.success ? <Check size={12} /> : <X size={12} />}
                    {result.message}
                </div>
            )}
        </div>
    );
};

const SettingInput: React.FC<{ label: string, value: string, placeholder?: string, onBlur: (v: string) => void, saving?: boolean }> = ({ label, value, placeholder, onBlur, saving }) => {
    const [local, setLocal] = useState(value);
    useEffect(() => setLocal(value), [value]);

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>{label}</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <input
                type="text"
                value={local}
                placeholder={placeholder}
                onChange={(e) => setLocal(e.target.value)}
                onBlur={() => onBlur(local)}
                style={inputStyle}
            />
        </div>
    );
};

// --- Styles ---

const labelStyle: React.CSSProperties = {
    display: 'block',
    color: 'var(--text-secondary)',
    fontSize: '0.8rem',
    marginBottom: '0.8rem',
    fontWeight: 500
};

const inputStyle: React.CSSProperties = {
    width: '100%',
    background: 'rgba(255,255,255,0.03)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    padding: '0.8rem',
    color: 'var(--text-primary)',
    fontSize: '0.85rem',
    outline: 'none',
    transition: 'all 0.2s',
    boxSizing: 'border-box'
};

const selectStyle: React.CSSProperties = {
    ...inputStyle,
    cursor: 'pointer',
    appearance: 'none',
    backgroundImage: 'linear-gradient(45deg, transparent 50%, gray 50%), linear-gradient(135deg, gray 50%, transparent 50%)',
    backgroundPosition: 'calc(100% - 20px) calc(1em + 2px), calc(100% - 15px) calc(1em + 2px)',
    backgroundSize: '5px 5px, 5px 5px',
    backgroundRepeat: 'no-repeat'
};

const charCardStyle = (active: boolean, tint: 'purple' | 'cyan'): React.CSSProperties => ({
    padding: '1.2rem',
    borderRadius: 'var(--radius-md)',
    background: active ? `var(--accent-${tint}-glass)` : 'rgba(255,255,255,0.02)',
    border: `1px solid ${active ? `var(--accent-${tint})` : 'var(--border-glass)'}`,
    cursor: 'pointer',
    transition: 'all 0.2s',
    textAlign: 'center',
    boxShadow: active ? `0 0 15px var(--accent-${tint}-glass)` : 'none'
});

const styleCardStyle = (active: boolean, character: string): React.CSSProperties => ({
    padding: '0.8rem',
    borderRadius: 'var(--radius-md)',
    background: active ? 'rgba(255,255,255,0.06)' : 'transparent',
    border: `1px solid ${active ? (character === 'male' ? 'var(--accent-cyan)' : 'var(--accent-purple)') : 'var(--border-glass)'}`,
    cursor: 'pointer',
    textAlign: 'center',
    fontSize: '0.8rem',
    transition: 'all 0.2s',
    color: active ? 'var(--text-bright)' : 'var(--text-muted)'
});

const modeBtnStyle = (active: boolean): React.CSSProperties => ({
    flex: 1,
    padding: '10px',
    border: 'none',
    background: active ? 'var(--accent-cyan)' : 'transparent',
    color: active ? '#000' : 'var(--text-muted)',
    borderRadius: '8px',
    cursor: 'pointer',
    fontSize: '0.8rem',
    fontWeight: active ? 700 : 400,
    textTransform: 'capitalize',
    transition: 'all 0.2s'
});

const testBtnStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    padding: '0.6rem 1rem',
    background: 'rgba(255,255,255,0.05)',
    border: '1px solid var(--border-glass)',
    borderRadius: 'var(--radius-sm)',
    color: 'var(--text-primary)',
    fontSize: '0.75rem',
    cursor: 'pointer',
    transition: 'all 0.2s'
};

const testResultStyle = (success: boolean): React.CSSProperties => ({
    marginTop: '0.6rem',
    fontSize: '0.7rem',
    color: success ? 'var(--accent-emerald)' : 'var(--accent-rose)',
    display: 'flex',
    alignItems: 'center',
    gap: '0.4rem',
    background: success ? 'rgba(16,185,129,0.05)' : 'rgba(255,77,148,0.05)',
    padding: '0.5rem',
    borderRadius: '4px'
});

const toggleBtnStyle = (active: boolean): React.CSSProperties => ({
    width: '40px',
    height: '22px',
    borderRadius: '11px',
    background: active ? 'var(--accent-cyan)' : 'rgba(255,255,255,0.1)',
    border: 'none',
    cursor: 'pointer',
    position: 'relative',
    transition: 'all 0.3s ease',
    padding: 0
});

const toggleCircleStyle = (active: boolean): React.CSSProperties => ({
    width: '16px',
    height: '16px',
    borderRadius: '50%',
    background: active ? '#000' : 'var(--text-muted)',
    position: 'absolute',
    top: '3px',
    left: active ? '21px' : '3px',
    transition: 'all 0.3s cubic-bezier(0.68, -0.55, 0.265, 1.55)'
});

const OllamaModelSelector: React.FC<{ value: string, onSelect: (v: string) => void, saving?: boolean }> = ({ value, onSelect, saving }) => {
    const [models, setModels] = useState<string[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState('');

    useEffect(() => {
        fetchModels();
    }, []);

    const fetchModels = async () => {
        setLoading(true);
        setError('');
        try {
            const res = await authenticatedFetch(`${API_BASE}/api/v1/ollama/models`);
            if (res.ok) {
                const data = await res.json();
                if (data.models && Array.isArray(data.models)) {
                    setModels(data.models.map((m: any) => m.name));
                }
            } else {
                setError(`Failed to fetch models: ${res.status}`);
            }
        } catch (err: any) {
            console.error("Fetch models error:", err);
            setError(`Connection error: ${err.message || 'Unknown error'}`);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.6rem' }}>
                <label style={{ ...labelStyle, marginBottom: 0 }}>Ollama Model</label>
                {saving && <Loader2 size={12} className="ani-spin" color="var(--accent-cyan)" />}
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
                <select
                    value={value}
                    onChange={(e) => onSelect(e.target.value)}
                    style={{ ...inputStyle, flex: 1, padding: '0.67rem', outline: 'none' }}
                >
                    <option value="">(Enter manually or select...)</option>
                    {models.map(m => (
                        <option key={m} value={m}>{m}</option>
                    ))}
                    {!models.includes(value) && value && (
                        <option value={value}>{value} (Current)</option>
                    )}
                </select>
                <button onClick={fetchModels} disabled={loading} title="Refresh Models" style={{ ...testBtnStyle, padding: '0.5rem 0.8rem' }}>
                    {loading ? <Loader2 size={14} className="ani-spin" /> : 'Refresh'}
                </button>
            </div>
            {error && <div style={{ fontSize: '0.7rem', color: 'var(--accent-rose)', marginTop: '0.4rem' }}>{error}</div>}
        </div>
    );
};

export default SettingsPage;
