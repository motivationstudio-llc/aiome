import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Lock, ShieldAlert, Zap, Loader2 } from 'lucide-react';
import { API_BASE } from '../config';
import { setAuthToken } from '../lib/auth';

interface AuthOverlayProps {
    onAuthenticated: () => void;
}

const AuthOverlay: React.FC<AuthOverlayProps> = ({ onAuthenticated }) => {
    const [token, setToken] = useState('');
    const [error, setError] = useState('');
    const [isLoading, setIsLoading] = useState(false);

    const handleLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!token) return;

        setIsLoading(true);
        setError('');

        try {
            // API Server のヘルスチェックを使用して検証 (KeyProxy も兼ねる)
            const response = await fetch(`${API_BASE}/api/health`, {
                headers: {
                    'Authorization': `Bearer ${token}`
                }
            });

            if (response.ok) {
                setAuthToken(token);
                onAuthenticated();
            } else {
                setError('Invalid Secret Key. Access Denied.');
            }
        } catch (err) {
            setError('Failed to connect to Abyss Vault. Ensure KeyProxy is running.');
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <motion.div 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="auth-overlay"
            style={{
                position: 'fixed',
                inset: 0,
                zIndex: 9999,
                background: 'rgba(0, 0, 0, 0.85)',
                backdropFilter: 'blur(20px)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '2rem'
            }}
        >
            <motion.div
                initial={{ scale: 0.9, y: 20 }}
                animate={{ scale: 1, y: 0 }}
                style={{
                    width: '100%',
                    maxWidth: '450px',
                    background: 'linear-gradient(135deg, rgba(20, 20, 25, 0.8) 0%, rgba(10, 10, 15, 0.9) 100%)',
                    border: '1px solid rgba(0, 242, 255, 0.1)',
                    borderRadius: '24px',
                    padding: '3rem',
                    textAlign: 'center',
                    boxShadow: '0 20px 50px rgba(0, 0, 0, 0.5), 0 0 30px rgba(0, 242, 255, 0.05)'
                }}
            >
                <div style={{ marginBottom: '2rem', display: 'flex', justifyContent: 'center' }}>
                    <div style={{ 
                        width: '80px', 
                        height: '80px', 
                        borderRadius: '50%', 
                        background: 'rgba(0, 242, 255, 0.05)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        border: '1px solid rgba(0, 242, 255, 0.2)'
                    }}>
                        <Lock color="#00f2ff" size={40} />
                    </div>
                </div>

                <h2 style={{ 
                    color: '#fff', 
                    fontSize: '1.8rem', 
                    fontWeight: 700, 
                    marginBottom: '0.5rem',
                    letterSpacing: '-0.02em'
                }}>
                    Abyss Vault Access
                </h2>
                <p style={{ 
                    color: 'rgba(255, 255, 255, 0.5)', 
                    fontSize: '0.9rem', 
                    marginBottom: '2rem' 
                }}>
                    Please enter your Aiome Secret to synchronize with the neural foundation.
                </p>

                <form onSubmit={handleLogin} style={{ textAlign: 'left' }}>
                    <div style={{ marginBottom: '1.5rem' }}>
                        <label style={{ 
                            display: 'block', 
                            color: 'rgba(0, 242, 255, 0.7)', 
                            fontSize: '0.75rem', 
                            textTransform: 'uppercase',
                            letterSpacing: '0.1em',
                            marginBottom: '0.5rem',
                            fontWeight: 600
                        }}>
                            Vault Secret Key
                        </label>
                        <input 
                            type="password"
                            value={token}
                            onChange={(e) => setToken(e.target.value)}
                            placeholder="••••••••••••••••"
                            style={{
                                width: '100%',
                                background: 'rgba(255, 255, 255, 0.03)',
                                border: '1px solid rgba(255, 255, 255, 0.1)',
                                borderRadius: '12px',
                                padding: '1rem',
                                color: '#fff',
                                outline: 'none',
                                fontSize: '1rem',
                                transition: 'all 0.2s'
                            }}
                            autoFocus
                        />
                    </div>

                    <AnimatePresence>
                        {error && (
                            <motion.div 
                                initial={{ opacity: 0, height: 0 }}
                                animate={{ opacity: 1, height: 'auto' }}
                                exit={{ opacity: 0, height: 0 }}
                                style={{ 
                                    display: 'flex', 
                                    alignItems: 'center', 
                                    gap: '0.75rem', 
                                    color: '#ff4d6d',
                                    fontSize: '0.85rem',
                                    marginBottom: '1.5rem',
                                    padding: '0.75rem',
                                    background: 'rgba(255, 77, 109, 0.05)',
                                    borderRadius: '10px',
                                    border: '1px solid rgba(255, 77, 109, 0.1)'
                                }}
                            >
                                <ShieldAlert size={16} />
                                <span>{error}</span>
                            </motion.div>
                        )}
                    </AnimatePresence>

                    <button
                        type="submit"
                        disabled={isLoading || !token}
                        style={{
                            width: '100%',
                            padding: '1rem',
                            borderRadius: '12px',
                            background: isLoading ? 'transparent' : 'linear-gradient(90deg, #00f2ff, #bc8cff)',
                            border: 'none',
                            color: '#000',
                            fontWeight: 700,
                            fontSize: '1rem',
                            cursor: (isLoading || !token) ? 'not-allowed' : 'pointer',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.5rem',
                            opacity: (isLoading || !token) ? 0.5 : 1,
                            transition: 'all 0.2s'
                        }}
                    >
                        {isLoading ? (
                            <Loader2 className="animate-spin" size={20} color="#00f2ff" />
                        ) : (
                            <>
                                <Zap size={20} />
                                Synchronize
                            </>
                        )}
                    </button>
                </form>

                <div style={{ marginTop: '2rem', fontSize: '0.75rem', color: 'rgba(255, 255, 255, 0.3)' }}>
                    Aiome Secure Gateway v1.0.2
                </div>
            </motion.div>
        </motion.div>
    );
};

export default AuthOverlay;
