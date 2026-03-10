import React, { useRef, useEffect, Suspense } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRM, VRMUtils } from '@pixiv/three-vrm';
import { MeshReflectorMaterial, Sparkles, Float } from '@react-three/drei';
import { useVrmExpression } from './useVrmExpression';

interface VrmModelProps {
    url: string;
    avatarState: string;
    onLoaded?: () => void;
    onError?: (err: Error) => void;
}

const VrmModel: React.FC<VrmModelProps> = ({ url, avatarState, onLoaded, onError }) => {
    const vrmRef = useRef<VRM | null>(null);
    const loadingRef = useRef<string | null>(null);
    const { scene } = useThree();
    const clockRef = useRef(new THREE.Clock());
    const initialHipsYRef = useRef<number | null>(null);

    // Apply separated expression logic
    useVrmExpression(vrmRef.current, avatarState);

    useEffect(() => {
        if (loadingRef.current === url) return;
        loadingRef.current = url;

        const loader = new GLTFLoader();
        loader.register((parser) => new VRMLoaderPlugin(parser));

        let isMounted = true;

        loader.load(
            url,
            (gltf) => {
                if (!isMounted) return;
                const vrm = gltf.userData.vrm as VRM;
                if (!vrm) {
                    onError?.(new Error('VRM data not found'));
                    return;
                }

                VRMUtils.removeUnnecessaryJoints(vrm.scene);
                VRMUtils.removeUnnecessaryVertices(vrm.scene);

                vrm.scene.rotation.y = Math.PI;
                // Position model: feet on reflector floor
                vrm.scene.position.set(0, -0.85, 0);

                scene.add(vrm.scene);
                vrmRef.current = vrm;

                if (vrm.humanoid) {
                    const hips = vrm.humanoid.getNormalizedBoneNode('hips');
                    if (hips) {
                        initialHipsYRef.current = hips.position.y;
                    }
                    const la = vrm.humanoid.getNormalizedBoneNode('leftUpperArm');
                    const ra = vrm.humanoid.getNormalizedBoneNode('rightUpperArm');
                    if (la) la.rotation.z = 1.2;
                    if (ra) ra.rotation.z = -1.2;
                }

                clockRef.current.start();
                onLoaded?.();
            },
            undefined,
            (error) => {
                if (!isMounted) return;
                console.error('Failed to load VRM:', error);
                onError?.(error instanceof Error ? error : new Error(String(error)));
                loadingRef.current = null;
            }
        );

        return () => {
            isMounted = false;
            loadingRef.current = null;
            if (vrmRef.current) {
                scene.remove(vrmRef.current.scene);
                VRMUtils.deepDispose(vrmRef.current.scene);
                vrmRef.current = null;
            }
        };
    }, [url, scene, onLoaded, onError]);

    useFrame((_, delta) => {
        if (!vrmRef.current) return;
        const elapsed = clockRef.current.getElapsedTime();
        if (vrmRef.current.humanoid) {
            const hips = vrmRef.current.humanoid.getNormalizedBoneNode('hips');
            if (hips && initialHipsYRef.current !== null) {
                hips.position.y = initialHipsYRef.current + Math.sin(elapsed * 1.5) * 0.003;
            }
        }
        vrmRef.current.update(delta);
    });

    return null;
};

interface VrmRendererProps {
    modelUrl: string;
    avatarState: string;
    onLoaded?: () => void;
    onError?: (err: Error) => void;
}

const VrmRenderer: React.FC<VrmRendererProps> = ({ modelUrl, avatarState, onLoaded, onError }) => {
    const isThinking = avatarState === 'thinking';
    const isSpeaking = avatarState === 'speaking';
    const isAwakened = avatarState === 'awakened';

    // Dynamic accent color
    const accentColor = isAwakened ? "#ffae00" : isThinking ? "#bc8cff" : "#00f2ff";

    return (
        <Canvas
            camera={{ position: [0, 0.9, 2.8], fov: 35 }}
            gl={{ alpha: true, antialias: true, preserveDrawingBuffer: true }}
            style={{ background: 'transparent' }}
            onCreated={({ gl }) => {
                gl.setClearColor(0x06080c, 1);
                gl.toneMapping = THREE.ACESFilmicToneMapping;
                gl.toneMappingExposure = 1.4;
            }}
        >
            <fog attach="fog" args={['#06080c', 3, 10]} />

            {/* === Lighting Setup === */}
            {/* Soft ambient fill */}
            <ambientLight intensity={0.3} color="#1a1a2e" />

            {/* Main key light — dramatic top-right */}
            <spotLight
                position={[3, 6, 4]}
                angle={0.2}
                penumbra={0.8}
                intensity={200}
                color={accentColor}
                castShadow
                shadow-mapSize-width={1024}
                shadow-mapSize-height={1024}
            />

            {/* Fill light — left side, softer */}
            <spotLight
                position={[-4, 3, 2]}
                angle={0.3}
                penumbra={1}
                intensity={80}
                color="#bc8cff"
            />

            {/* Rim light — back, creates edge glow */}
            <pointLight position={[0, 3, -3]} intensity={40} color="#00f2ff" />

            {/* Under-glow for dramatic effect */}
            <pointLight position={[0, -1, 1]} intensity={15} color={accentColor} />

            {/* === VRM Character with Float === */}
            <Float speed={1.0} rotationIntensity={0.015} floatIntensity={0.03}>
                <Suspense fallback={null}>
                    <VrmModel url={modelUrl} avatarState={avatarState} onLoaded={onLoaded} onError={onError} />
                </Suspense>
            </Float>

            {/* === Reflector Floor === */}
            <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -0.62, 0]}>
                <planeGeometry args={[30, 30]} />
                <MeshReflectorMaterial
                    blur={[400, 200]}
                    resolution={1024}
                    mixBlur={1}
                    mixStrength={80}
                    roughness={0.85}
                    depthScale={1.5}
                    minDepthThreshold={0.3}
                    maxDepthThreshold={1.5}
                    color="#080808"
                    metalness={0.6}
                    mirror={0.15}
                />
            </mesh>

            {/* === Particles === */}
            {/* Main ambient sparkles */}
            <Sparkles
                count={isAwakened ? 120 : 50}
                scale={[6, 4, 6]}
                size={isAwakened ? 3 : 2}
                speed={0.3}
                color={accentColor}
                opacity={0.5}
            />
            {/* Secondary subtle particles */}
            <Sparkles
                count={30}
                scale={[4, 3, 4]}
                size={1}
                speed={0.15}
                color="#ffffff"
                opacity={0.15}
            />
        </Canvas>
    );
};

export default VrmRenderer;
