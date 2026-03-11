import React from 'react';
import { motion } from 'framer-motion';
import { useAvatarCharacter } from '../hooks/AvatarContext';

interface AiomeAvatarProps {
    status: 'idle' | 'thinking' | 'awakened';
    size?: number;
}

const AiomeAvatar: React.FC<AiomeAvatarProps> = ({ status, size = 120 }) => {
    const { getAssetPath } = useAvatarCharacter();
    const imagePath = getAssetPath('lite');

    return (
        <div style={{
            width: size,
            height: size,
            position: 'relative',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            mixBlendMode: 'screen', // Blend the entire container
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

            {/* Main Character Image */}
            <motion.div
                animate={{
                    y: status === 'thinking' ? [-5, 5, -5] : [0, 0],
                    filter: status === 'thinking'
                        ? ['drop-shadow(0 0 10px rgba(0, 242, 255, 0.6))', 'drop-shadow(0 0 30px rgba(0, 242, 255, 1))', 'drop-shadow(0 0 10px rgba(0, 242, 255, 0.6))']
                        : ['drop-shadow(0 0 10px rgba(0, 242, 255, 0.3))', 'drop-shadow(0 0 20px rgba(0, 242, 255, 0.3))']
                }}
                transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
                style={{
                    width: '90%',
                    height: '90%',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 2,
                }}
            >
                <img
                    src={imagePath}
                    alt="Aiome Chibi"
                    style={{
                        width: '100%',
                        height: '100%',
                        objectFit: 'contain',
                        filter: 'contrast(1.2) brightness(1.1) drop-shadow(0 0 15px rgba(0, 242, 255, 0.4))',
                        maskImage: 'radial-gradient(circle, black 50%, transparent 85%)', // Sharper mask to hide edges
                        WebkitMaskImage: 'radial-gradient(circle, black 50%, transparent 85%)',
                    }}
                />
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
