import { useState, useEffect } from 'react';

export type DisplayMode = 'vrm' | 'lite' | 'off';

export const useDisplayMode = () => {
    const [mode, setMode] = useState<DisplayMode>(() => {
        const saved = localStorage.getItem('aiome_display_mode');
        // Migrate old 'live2d' value to 'vrm'
        if (saved === 'live2d') return 'vrm';
        return (saved as DisplayMode) || 'vrm';
    });

    useEffect(() => {
        localStorage.setItem('aiome_display_mode', mode);
    }, [mode]);

    return { mode, setMode };
};
