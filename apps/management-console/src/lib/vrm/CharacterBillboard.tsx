import React, { useRef, useMemo } from 'react';
import { useFrame, useLoader } from '@react-three/fiber';
import * as THREE from 'three';

interface CharacterBillboardProps {
    url: string;
    avatarState: string;
}

const CharacterBillboard: React.FC<CharacterBillboardProps> = ({ url, avatarState }) => {
    const texture = useLoader(THREE.TextureLoader, url);
    const meshRef = useRef<THREE.Mesh>(null);
    const materialRef = useRef<THREE.MeshBasicMaterial>(null);

    const isThinking = avatarState === 'thinking';
    const isAwakened = avatarState === 'awakened';
    const isLearning = avatarState === 'learning';
    const isSpeaking = avatarState === 'speaking';

    // Dynamic accent color for the hologram
    const accentColor = useMemo(() => {
        if (isAwakened) return new THREE.Color("#ffae00");
        if (isThinking) return new THREE.Color("#bc8cff");
        if (isLearning) return new THREE.Color("#00ff88");
        return new THREE.Color("#00f2ff");
    }, [isAwakened, isThinking, isLearning]);

    useFrame((state) => {
        if (!meshRef.current) return;
        const t = state.clock.getElapsedTime();

        // Face the camera (cylindrical billboard)
        meshRef.current.rotation.y = state.camera.rotation.y;
        meshRef.current.rotation.x = 0;
        meshRef.current.rotation.z = 0;

        // 1. Floating & Breathing
        const floatY = Math.sin(t * 1.5) * 0.03;
        const breathScale = 1 + Math.sin(t * 2) * 0.01;

        meshRef.current.position.y = floatY;
        meshRef.current.scale.set(1.0 * breathScale, 1.0 * breathScale, 1);

        // 2. Status-specific animations
        if (isThinking) {
            meshRef.current.position.x = Math.sin(t * 10) * 0.005; // Nervous twitch
        }

        if (isSpeaking) {
            const talkScale = 1 + Math.sin(t * 15) * 0.01;
            meshRef.current.scale.y *= talkScale;
        }

        // 3. Material Updates
        if (materialRef.current) {
            // Pulse the opacity slightly
            materialRef.current.opacity = 0.85 + Math.sin(t * 3) * 0.05;

            // Subtle color tint shift
            materialRef.current.color.lerp(accentColor, 0.1);
        }
    });

    return (
        <group position={[0, 0.35, 0]}>
            {/* Main Holographic Mesh (Cylindrical Billboard) - Scaled down to ensure no clipping */}
            <mesh ref={meshRef}>
                <planeGeometry args={[1.6, 1.6]} />
                <meshBasicMaterial
                    ref={materialRef}
                    map={texture}
                    transparent={true}
                    opacity={0.9}
                    depthWrite={false}
                    toneMapped={false}
                    blending={THREE.AdditiveBlending}
                    side={THREE.DoubleSide}
                />
            </mesh>

            {/* Subtle Back Glow */}
            <mesh position={[0, 0, -0.1]}>
                <planeGeometry args={[2.0, 2.0]} />
                <meshBasicMaterial
                    color={accentColor}
                    transparent={true}
                    opacity={0.05}
                    blending={THREE.AdditiveBlending}
                />
            </mesh>

            {/* Pedestal Base Rings - Placed exactly on the floor (-0.62 relative to world) */}
            <group position={[0, -0.97, 0]} rotation={[-Math.PI / 2, 0, 0]}>
                <mesh>
                    <ringGeometry args={[0.5, 0.52, 64]} />
                    <meshBasicMaterial color={accentColor} transparent opacity={0.3} blending={THREE.AdditiveBlending} />
                </mesh>
                <mesh position={[0, 0, 0.05]}>
                    <ringGeometry args={[0.3, 0.32, 64]} />
                    <meshBasicMaterial color={accentColor} transparent opacity={0.5} blending={THREE.AdditiveBlending} />
                </mesh>
            </group>
        </group>
    );
};

export default CharacterBillboard;
