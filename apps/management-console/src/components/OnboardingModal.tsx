import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { BrainCircuit, Sparkles, Shield, User, UserCheck } from 'lucide-react';
import { useAvatarCharacter } from '../hooks/AvatarContext';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';

interface OnboardingModalProps {
    isOpen: boolean;
    onClose: () => void;
}

const OnboardingModal: React.FC<OnboardingModalProps> = ({ isOpen, onClose }) => {
    const [step, setStep] = useState(0);
    const [aiName, setAiName] = useState("Watchtower");
    const { character, setCharacter, proportion, setProportion } = useAvatarCharacter();
    const [isSaving, setIsSaving] = useState(false);

    const handleFinalize = async () => {
        setIsSaving(true);
        try {
            // Save AI Name to DB
            await fetch(`${API_BASE}/api/v1/settings`, {
                method: 'PUT',
                headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
                body: JSON.stringify({ key: 'ai_name', value: aiName, category: 'identity' })
            });
            // Avatar settings are already handled by context (localStorage)
            onClose();
        } catch (error) {
            console.error("Failed to save onboarding settings", error);
            onClose();
        } finally {
            setIsSaving(false);
        }
    };

    const steps = [
        {
            title: "Welcome to Aiome",
            description: "The autonomous AI Operating System designed for the next era of agency.",
            icon: <BrainCircuit size={48} color="var(--accent-cyan)" />,
        },
        {
            title: "Name Your AI",
            description: "What should your system manifestation call itself?",
            icon: <User size={48} color="var(--accent-cyan)" />,
            content: (
                <div style={{ marginTop: '1rem', width: '100%' }}>
                    <input
                        type="text"
                        value={aiName}
                        onChange={(e) => setAiName(e.target.value)}
                        placeholder="e.g. Watchtower, Genesis, Luna..."
                        style={{
                            width: '100%',
                            background: 'rgba(255,255,255,0.05)',
                            border: '1px solid var(--border-glass-bright)',
                            borderRadius: '12px',
                            padding: '1rem',
                            color: '#fff',
                            fontSize: '1.2rem',
                            textAlign: 'center',
                            outline: 'none'
                        }}
                    />
                </div>
            )
        },
        {
            title: "Choose Manifestation",
            description: "Select the visual form of your AI presence.",
            icon: <UserCheck size={48} color="var(--accent-purple)" />,
            content: (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', width: '100%', marginTop: '0.5rem' }}>
                    <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                         <button 
                            onClick={() => setCharacter('female')}
                            style={{ 
                                flex: 1, padding: '1rem', borderRadius: '12px', 
                                border: `2px solid ${character === 'female' ? 'var(--accent-purple)' : 'transparent'}`,
                                background: character === 'female' ? 'rgba(168, 85, 247, 0.1)' : 'rgba(255,255,255,0.03)',
                                cursor: 'pointer', transition: 'all 0.2s ease'
                            }}
                         >
                             <div style={{ fontSize: '1.5rem', marginBottom: '0.2rem' }}>♀</div>
                             <div style={{ fontSize: '0.8rem', fontWeight: 600 }}>Female</div>
                         </button>
                         <button 
                            onClick={() => setCharacter('male')}
                            style={{ 
                                flex: 1, padding: '1rem', borderRadius: '12px', 
                                border: `2px solid ${character === 'male' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: character === 'male' ? 'rgba(34, 211, 238, 0.1)' : 'rgba(255,255,255,0.03)',
                                cursor: 'pointer', transition: 'all 0.2s ease'
                            }}
                         >
                             <div style={{ fontSize: '1.5rem', marginBottom: '0.2rem' }}>♂</div>
                             <div style={{ fontSize: '0.8rem', fontWeight: 600 }}>Male</div>
                         </button>
                    </div>
                    <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                         <button 
                            onClick={() => setProportion('chibi')}
                            style={{ 
                                flex: 1, padding: '0.8rem', borderRadius: '10px', 
                                border: `2px solid ${proportion === 'chibi' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: proportion === 'chibi' ? 'rgba(34, 211, 238, 0.05)' : 'rgba(255,255,255,0.03)',
                                fontSize: '0.8rem', cursor: 'pointer'
                            }}
                         >
                             Cute Chibi
                         </button>
                         <button 
                            onClick={() => setProportion('taller')}
                            style={{ 
                                flex: 1, padding: '0.8rem', borderRadius: '10px', 
                                border: `2px solid ${proportion === 'taller' ? 'var(--accent-cyan)' : 'transparent'}`,
                                background: proportion === 'taller' ? 'rgba(34, 211, 238, 0.05)' : 'rgba(255,255,255,0.03)',
                                fontSize: '0.8rem', cursor: 'pointer'
                            }}
                         >
                             Modern Taller
                         </button>
                    </div>
                </div>
            )
        },
        {
            title: "Abyss Vault Security",
            description: "Your API keys are physically isolated at the OS level, ensuring safety even in autonomous mode.",
            icon: <Shield size={48} color="var(--accent-rose)" />,
        },
    ];

    return (
        <AnimatePresence>
            {isOpen && (
                <motion.div
                    className="modal-overlay"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    style={{
                        position: 'fixed',
                        inset: 0,
                        background: 'rgba(0, 0, 0, 0.85)',
                        backdropFilter: 'blur(20px)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        zIndex: 1000,
                    }}
                >
                    <motion.div
                        initial={{ scale: 0.9, opacity: 0, y: 20 }}
                        animate={{ scale: 1, opacity: 1, y: 0 }}
                        exit={{ scale: 0.9, opacity: 0, y: 20 }}
                        className="modal-container"
                        style={{
                            width: '500px',
                            minHeight: '520px',
                            padding: '3rem',
                            background: 'var(--bg-glass-heavy)',
                            border: '1px solid var(--border-glass-bright)',
                            borderRadius: 'var(--radius-xl)',
                            textAlign: 'center',
                            boxShadow: 'var(--shadow-deep)',
                            display: 'flex',
                            flexDirection: 'column'
                        }}
                    >
                        <div style={{ flex: 1 }}>
                            <AnimatePresence mode="wait">
                                <motion.div
                                    key={step}
                                    initial={{ x: 20, opacity: 0 }}
                                    animate={{ x: 0, opacity: 1 }}
                                    exit={{ x: -20, opacity: 0 }}
                                    style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.2rem' }}
                                >
                                    <div style={{ padding: '2rem', background: 'rgba(255,255,255,0.03)', borderRadius: '50%', marginBottom: '0.5rem' }}>
                                        {steps[step].icon}
                                    </div>
                                    <h2 style={{ fontSize: '1.8rem', fontWeight: 800 }}>{steps[step].title}</h2>
                                    <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, fontSize: '1rem' }}>
                                        {steps[step].description}
                                    </p>
                                    {steps[step].content}
                                </motion.div>
                            </AnimatePresence>
                        </div>

                        <div style={{ marginTop: '2rem' }}>
                            <div style={{ display: 'flex', justifyContent: 'center', gap: '1rem' }}>
                                {step < steps.length - 1 ? (
                                    <button
                                        onClick={() => setStep(step + 1)}
                                        style={{
                                            padding: '0.8rem 2.5rem',
                                            background: 'var(--accent-cyan)',
                                            color: '#000',
                                            border: 'none',
                                            borderRadius: 'var(--radius-md)',
                                            fontWeight: 700,
                                            cursor: 'pointer',
                                        }}
                                    >
                                        Continue
                                    </button>
                                ) : (
                                    <button
                                        onClick={handleFinalize}
                                        disabled={isSaving}
                                        style={{
                                            padding: '0.8rem 2.5rem',
                                            background: 'linear-gradient(135deg, var(--accent-cyan), var(--accent-purple))',
                                            color: '#fff',
                                            border: 'none',
                                            borderRadius: 'var(--radius-md)',
                                            fontWeight: 700,
                                            cursor: 'pointer',
                                            display: 'flex',
                                            alignItems: 'center',
                                            gap: '0.5rem',
                                            opacity: isSaving ? 0.7 : 1
                                        }}
                                    >
                                        <Sparkles size={20} />
                                        {isSaving ? "Finalizing..." : "Awaken System"}
                                    </button>
                                )}
                            </div>

                            <div style={{ marginTop: '2rem', display: 'flex', justifyContent: 'center', gap: '0.5rem' }}>
                                {steps.map((_, i) => (
                                    <div
                                        key={i}
                                        style={{
                                            width: '8px',
                                            height: '8px',
                                            borderRadius: '50%',
                                            background: i === step ? 'var(--accent-cyan)' : 'var(--text-muted)',
                                            transition: 'all 0.3s ease'
                                        }}
                                    />
                                ))}
                            </div>
                        </div>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};

export default OnboardingModal;
