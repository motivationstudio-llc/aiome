import React, { useRef, useEffect, Suspense } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';

import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRM, VRMUtils } from '@pixiv/three-vrm';

interface VrmModelProps {
    url: string;
    avatarState: string;
    onLoaded?: () => void;
    onError?: (err: Error) => void;
}

const VrmModel: React.FC<VrmModelProps> = ({ url, avatarState, onLoaded, onError }) => {
    const vrmRef = useRef<VRM | null>(null);
    const { scene } = useThree();
    const clockRef = useRef(new THREE.Clock());
    const blinkTimerRef = useRef(0);
    const lipIndexRef = useRef(0);
    const lipTimerRef = useRef(0);

    useEffect(() => {
        const loader = new GLTFLoader();
        loader.register((parser) => new VRMLoaderPlugin(parser));

        loader.load(
            url,
            (gltf) => {
                const vrm = gltf.userData.vrm as VRM;
                if (!vrm) {
                    onError?.(new Error('VRM data not found in glTF'));
                    return;
                }

                VRMUtils.removeUnnecessaryJoints(vrm.scene);
                VRMUtils.removeUnnecessaryVertices(vrm.scene);

                // Rotate model to face camera
                vrm.scene.rotation.y = Math.PI;

                // Position and scale: Raise to center of camera
                vrm.scene.position.set(0, 0.4, 0);
                const scale = 1.0;
                vrm.scene.scale.set(scale, scale, scale);

                scene.add(vrm.scene);
                vrmRef.current = vrm;

                // Remove T-pose (drop arms)
                if (vrm.humanoid) {
                    const leftUpperArm = vrm.humanoid.getNormalizedBoneNode('leftUpperArm');
                    const rightUpperArm = vrm.humanoid.getNormalizedBoneNode('rightUpperArm');
                    if (leftUpperArm) leftUpperArm.rotation.z = 1.2;
                    if (rightUpperArm) rightUpperArm.rotation.z = -1.2;
                }

                clockRef.current.start();

                onLoaded?.();
                console.log('VRM model loaded successfully');
            },
            undefined,
            (error) => {
                console.error('Failed to load VRM:', error);
                onError?.(error instanceof Error ? error : new Error(String(error)));
            }
        );

        return () => {
            if (vrmRef.current) {
                scene.remove(vrmRef.current.scene);
                VRMUtils.deepDispose(vrmRef.current.scene);
                vrmRef.current = null;
            }
        };
    }, [url, scene]);

    useFrame(() => {
        const vrm = vrmRef.current;
        if (!vrm) return;

        const delta = clockRef.current.getDelta();
        const elapsed = clockRef.current.getElapsedTime();

        // --- Auto Blink ---
        blinkTimerRef.current -= delta;
        if (blinkTimerRef.current <= 0) {
            // Trigger blink
            const blinkDuration = 0.15;
            vrm.expressionManager?.setValue('blink', 1);
            setTimeout(() => {
                vrm.expressionManager?.setValue('blink', 0);
            }, blinkDuration * 1000);
            // Next blink in 3-6 seconds
            blinkTimerRef.current = 3 + Math.random() * 3;
        }

        // --- Breathing Animation ---
        if (vrm.humanoid) {
            const hips = vrm.humanoid.getNormalizedBoneNode('hips');
            if (hips) {
                hips.position.y = -0.8 + Math.sin(elapsed * 1.5) * 0.003;
            }
        }

        // --- State-based Expressions ---
        // Reset all expressions first
        vrm.expressionManager?.setValue('happy', 0);
        vrm.expressionManager?.setValue('angry', 0);
        vrm.expressionManager?.setValue('sad', 0);
        vrm.expressionManager?.setValue('relaxed', 0);
        vrm.expressionManager?.setValue('surprised', 0);
        vrm.expressionManager?.setValue('aa', 0);
        vrm.expressionManager?.setValue('ih', 0);
        vrm.expressionManager?.setValue('ou', 0);
        vrm.expressionManager?.setValue('ee', 0);
        vrm.expressionManager?.setValue('oh', 0);

        switch (avatarState) {
            case 'thinking':
                vrm.expressionManager?.setValue('surprised', 0.6);
                // Slight head tilt
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    if (head) {
                        head.rotation.z = Math.sin(elapsed * 0.5) * 0.08;
                    }
                }
                break;

            case 'speaking': {
                vrm.expressionManager?.setValue('happy', 0.3);
                // Lip sync: cycle through vowels
                lipTimerRef.current -= delta;
                if (lipTimerRef.current <= 0) {
                    const vowels = ['aa', 'ih', 'ou', 'ee', 'oh'];
                    const vowel = vowels[lipIndexRef.current % vowels.length];
                    vrm.expressionManager?.setValue(vowel, 0.7 + Math.random() * 0.3);
                    lipIndexRef.current++;
                    lipTimerRef.current = 0.1 + Math.random() * 0.15;
                }
                break;
            }

            case 'learning':
                vrm.expressionManager?.setValue('relaxed', 0.7);
                // Nodding animation
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    if (head) {
                        head.rotation.x = Math.sin(elapsed * 2) * 0.06;
                    }
                }
                break;

            case 'meditating':
                vrm.expressionManager?.setValue('blink', 0.9); // Eyes mostly closed
                vrm.expressionManager?.setValue('relaxed', 0.5);
                break;

            case 'awakened':
                vrm.expressionManager?.setValue('happy', 0.9);
                vrm.expressionManager?.setValue('surprised', 0.3);
                break;

            default: // idle
                // Slight look-around
                if (vrm.humanoid) {
                    const head = vrm.humanoid.getNormalizedBoneNode('head');
                    if (head) {
                        head.rotation.y = Math.sin(elapsed * 0.3) * 0.05;
                        head.rotation.x = Math.sin(elapsed * 0.2) * 0.02;
                        head.rotation.z = 0;
                    }
                }
                break;
        }

        // Update VRM (spring bones, expression manager, etc.)
        vrm.update(delta);
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
    return (
        <Canvas
            camera={{ position: [0, 0.9, 2.5], fov: 30 }}
            gl={{ alpha: true, antialias: true, preserveDrawingBuffer: true }}
            style={{ background: 'rgba(11, 11, 15, 0.85)' }}
            onCreated={({ gl }) => {
                gl.setClearColor(0x0b0b0f, 0.85);
                gl.toneMapping = THREE.ACESFilmicToneMapping;
                gl.toneMappingExposure = 1.5;
            }}
        >
            {/* Lighting */}
            <ambientLight intensity={1.0} color="#ffffff" />
            <directionalLight position={[3, 5, 2]} intensity={2.0} color="#ffffff" />
            <directionalLight position={[-2, 3, -1]} intensity={0.8} color="#00f2ff" />
            <pointLight position={[0, 2, 3]} intensity={1.0} color="#bc8cff" distance={10} />

            {/* VRM Model */}
            <Suspense fallback={null}>
                <VrmModel
                    url={modelUrl}
                    avatarState={avatarState}
                    onLoaded={onLoaded}
                    onError={onError}
                />
            </Suspense>
        </Canvas>
    );
};

export default VrmRenderer;
