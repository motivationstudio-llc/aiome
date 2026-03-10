import React from 'react';
import { motion } from 'framer-motion';

interface AiomeAvatarProps {
    status: 'idle' | 'thinking' | 'awakened';
    size?: number;
}

const AiomeAvatar: React.FC<AiomeAvatarProps> = ({ status, size = 120 }) => {
    return (
        <div style={{
            width: size,
            height: size,
            position: 'relative',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
        }}>
            {/* Outer Glow Halo */}
            <motion.div
                animate={{
                    scale: status === 'thinking' ? [1, 1.2, 1] : status === 'awakened' ? [1, 1.4, 1.2] : 1,
                    opacity: status === 'thinking' ? [0.3, 0.6, 0.3] : 0.4,
                    rotate: [0, 360]
                }}
                transition={{
                    scale: { duration: 2, repeat: Infinity, ease: "easeInOut" },
                    opacity: { duration: 2, repeat: Infinity, ease: "easeInOut" },
                    rotate: { duration: 10, repeat: Infinity, ease: "linear" }
                }}
                style={{
                    position: 'absolute',
                    inset: -size * 0.2,
                    border: '2px dashed var(--accent-cyan)',
                    borderRadius: '50%',
                    filter: 'blur(4px)',
                    opacity: 0.3
                }}
            />

            {/* Main Core Sphere */}
            <motion.div
                animate={{
                    boxShadow: status === 'thinking'
                        ? ['0 0 20px rgba(0, 242, 255, 0.4)', '0 0 50px rgba(0, 242, 255, 0.8)', '0 0 20px rgba(0, 242, 255, 0.4)']
                        : ['0 0 20px rgba(0, 242, 255, 0.2)', '0 0 30px rgba(0, 242, 255, 0.2)']
                }}
                transition={{ duration: 1.5, repeat: Infinity }}
                style={{
                    width: '70%',
                    height: '70%',
                    borderRadius: '50%',
                    background: 'radial-gradient(circle at 30% 30%, #fff, var(--accent-cyan) 40%, var(--bg-dark-obsidian) 100%)',
                    position: 'relative',
                    overflow: 'hidden',
                    zIndex: 2,
                }}
            >
                {/* Inner neural structures / patterns */}
                <motion.div
                    animate={{
                        y: [-10, 10, -10],
                        opacity: [0.4, 0.8, 0.4]
                    }}
                    transition={{ duration: 4, repeat: Infinity, ease: "easeInOut" }}
                    style={{
                        position: 'absolute',
                        inset: 0,
                        background: 'url("data:image/svg+xml,%3Csvg width=\'20\' height=\'20\' viewBox=\'0 0 20 20\' xmlns=\'http://www.w3.org/2000/svg\'%3E%3Cpath d=\'M0 0h20L0 20z\' fill=\'%2300f2ff\' fill-opacity=\'.1\'/%3E%3C/svg%3E")',
                    }}
                />

                {/* Pulsing highlights */}
                {status === 'thinking' && (
                    <motion.div
                        animate={{ top: ['100%', '-100%'] }}
                        transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                        style={{
                            position: 'absolute',
                            width: '200%',
                            height: '40px',
                            background: 'linear-gradient(rgba(255,255,255,0), rgba(255,255,255,0.4), rgba(255,255,255,0))',
                            transform: 'rotate(-45deg)',
                            left: '-50%'
                        }}
                    />
                )}
            </motion.div>

            {/* Orbital Particles */}
            {[...Array(3)].map((_, i) => (
                <motion.div
                    key={i}
                    animate={{
                        rotate: [0, 360],
                    }}
                    transition={{
                        duration: 3 + i * 2,
                        repeat: Infinity,
                        ease: "linear"
                    }}
                    style={{
                        position: 'absolute',
                        inset: -i * 10,
                        pointerEvents: 'none'
                    }}
                >
                    <motion.div
                        animate={{ scale: [1, 1.5, 1] }}
                        transition={{ duration: 2, repeat: Infinity, delay: i * 0.5 }}
                        style={{
                            width: 6,
                            height: 6,
                            background: i === 1 ? 'var(--accent-purple)' : 'var(--accent-cyan)',
                            borderRadius: '50%',
                            top: 0,
                            left: '50%',
                            position: 'absolute',
                            boxShadow: '0 0 10px currentColor'
                        }}
                    />
                </motion.div>
            ))}
        </div>
    );
};

export default AiomeAvatar;
