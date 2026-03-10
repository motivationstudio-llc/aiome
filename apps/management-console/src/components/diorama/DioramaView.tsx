import React, { useState } from 'react';
import { useDisplayMode } from '../../hooks/useDisplayMode';
import AiomeAvatar from '../AiomeAvatar';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import ErrorBoundary from '../common/ErrorBoundary';

interface DioramaViewProps {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
}

const DioramaView: React.FC<DioramaViewProps> = ({ status }) => {
    const { mode } = useDisplayMode();
    const [hasError, setHasError] = useState(false);
    const [isLoading, setIsLoading] = useState(true);

    if (mode === 'off') return null;

    if (mode === 'lite' || hasError) {
        const liteStatus: 'idle' | 'thinking' | 'awakened' =
            (status === 'thinking' || status === 'learning' || status === 'speaking') ? 'thinking' :
                (status === 'awakened') ? 'awakened' : 'idle';
        return (
            <div style={{ position: 'fixed', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 0, pointerEvents: 'none' }}>
                <AiomeAvatar status={liteStatus} size={300} />
            </div>
        );
    }

    // VRM mode
    return (
        <div style={{ position: 'fixed', inset: 0, zIndex: 0, overflow: 'hidden', pointerEvents: 'none' }}>
            <ErrorBoundary
                fallback={null}
                onError={() => {
                    console.error('Canvas crash detected, falling back to lite mode');
                    setHasError(true);
                }}
            >
                <VrmRenderer
                    modelUrl="/vrm/sample/sample.vrm"
                    avatarState={status}
                    onLoaded={() => setIsLoading(false)}
                    onError={() => {
                        console.warn('VRM load failed, falling back to lite mode');
                        setHasError(true);
                        setIsLoading(false);
                    }}
                />
            </ErrorBoundary>
            {isLoading && (
                <div style={{
                    position: 'absolute', inset: 0,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'transparent',
                    color: 'var(--accent-cyan)',
                    fontSize: '0.85rem',
                    fontWeight: 600,
                    letterSpacing: '0.1em',
                    pointerEvents: 'none'
                }}>
                    <div style={{
                        padding: '0.8rem 1.5rem',
                        background: 'rgba(0,0,0,0.4)',
                        backdropFilter: 'blur(8px)',
                        borderRadius: '12px',
                        border: '1px solid rgba(0,242,255,0.15)'
                    }}>
                        ⏳ Initializing Aiome...
                    </div>
                </div>
            )}
        </div>
    );
};

export default DioramaView;
