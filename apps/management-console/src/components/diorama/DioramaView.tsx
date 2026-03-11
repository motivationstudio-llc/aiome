import React, { useState } from 'react';
import AiomeAvatar from '../AiomeAvatar';
import VrmRenderer from '../../lib/vrm/VrmRenderer';
import ErrorBoundary from '../common/ErrorBoundary';
import { useAvatarCharacter } from '../../hooks/AvatarContext';

interface DioramaViewProps {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    mode: 'vrm' | 'lite' | 'off';
    activeTab: string;
}

const DioramaView: React.FC<DioramaViewProps> = ({ status, mode, activeTab }) => {
    const [hasError, setHasError] = useState(false);
    const { getAssetPath } = useAvatarCharacter();
    const modelUrl = getAssetPath('vrm');

    // Layout offsets are derived from CSS custom properties defined in tokens.css.
    // This ensures DioramaView always aligns with the main content area
    // regardless of layout changes. Only tokens.css needs to be updated.
    const isDashboard = activeTab === "dashboard";
    const leftOffset = "calc(var(--layout-sidebar-width) + var(--layout-main-padding))";
    const rightOffset = isDashboard
        ? "calc(var(--layout-main-padding) + var(--layout-right-panel-width) + var(--layout-panel-gap))"
        : "var(--layout-main-padding)";

    // Reset error state when mode is manually changed by user
    React.useEffect(() => {
        setHasError(false);
    }, [mode]);

    if (mode === 'off') return null;

    if (mode === 'lite' || hasError) {
        const liteStatus: 'idle' | 'thinking' | 'awakened' =
            (status === 'thinking' || status === 'learning' || status === 'speaking') ? 'thinking' :
                (status === 'awakened') ? 'awakened' : 'idle';
        return (
            <div style={{ position: 'fixed', top: 0, bottom: 0, left: leftOffset, right: rightOffset, display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 0, pointerEvents: 'none', transform: 'translateY(11vh)' }}>
                <AiomeAvatar status={liteStatus} size={400} />
            </div>
        );
    }

    // Billboard mode
    return (
        <div style={{ position: 'fixed', top: 0, bottom: 0, left: leftOffset, right: rightOffset, zIndex: 0, overflow: 'hidden', pointerEvents: 'none', transform: 'translateY(11vh)' }}>
            <ErrorBoundary
                fallback={null}
                onError={() => {
                    console.error('Canvas crash detected, falling back to lite mode');
                    setHasError(true);
                }}
            >
                <VrmRenderer
                    modelUrl={modelUrl}
                    avatarState={status}
                />
            </ErrorBoundary>
        </div>
    );
};

export default DioramaView;
