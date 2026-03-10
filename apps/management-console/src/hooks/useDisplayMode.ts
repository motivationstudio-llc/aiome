import { useState, useEffect } from 'react';

export type DisplayMode = 'vrm' | 'lite' | 'off';

export const useDisplayMode = () => {
    const [mode, setMode] = useState<DisplayMode>(() => {
        const saved = localStorage.getItem('aiome_display_mode');
        if (saved === 'vrm' || saved === 'lite' || saved === 'off') {
            return saved;
        }
        return 'vrm';
    });

    useEffect(() => {
        localStorage.setItem('aiome_display_mode', mode);
    }, [mode]);

    return { mode, setMode };
};
