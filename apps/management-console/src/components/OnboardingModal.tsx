import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { BrainCircuit, Sparkles, Shield, Orbit } from 'lucide-react';

interface OnboardingModalProps {
    isOpen: boolean;
    onClose: () => void;
}

const OnboardingModal: React.FC<OnboardingModalProps> = ({ isOpen, onClose }) => {
    const [step, setStep] = React.useState(0);

    const steps = [
        {
            title: "Welcome to Aiome",
            description: "The autonomous AI Operating System designed for the next era of agency.",
            icon: <BrainCircuit size={48} color="var(--accent-cyan)" />,
        },
        {
            title: "Autonomous Evolution",
            description: "Every failure is captured as 'Karma', allowing your AI to learn and evolve independently.",
            icon: <Orbit size={48} color="var(--accent-purple)" />,
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
                            padding: '3rem',
                            background: 'var(--bg-glass-heavy)',
                            border: '1px solid var(--border-glass-bright)',
                            borderRadius: 'var(--radius-xl)',
                            textAlign: 'center',
                            boxShadow: 'var(--shadow-deep)',
                        }}
                    >
                        <AnimatePresence mode="wait">
                            <motion.div
                                key={step}
                                initial={{ x: 20, opacity: 0 }}
                                animate={{ x: 0, opacity: 1 }}
                                exit={{ x: -20, opacity: 0 }}
                                style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1.5rem' }}
                            >
                                <div style={{ padding: '2rem', background: 'rgba(255,255,255,0.03)', borderRadius: '50%', marginBottom: '1rem' }}>
                                    {steps[step].icon}
                                </div>
                                <h2 style={{ fontSize: '2rem', fontWeight: 800 }}>{steps[step].title}</h2>
                                <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, fontSize: '1.1rem' }}>
                                    {steps[step].description}
                                </p>
                            </motion.div>
                        </AnimatePresence>

                        <div style={{ marginTop: '3rem', display: 'flex', justifyContent: 'center', gap: '1rem' }}>
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
                                    onClick={onClose}
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
                                    }}
                                >
                                    <Sparkles size={20} />
                                    Awaken System
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
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};

export default OnboardingModal;
