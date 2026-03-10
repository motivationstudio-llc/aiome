import { useState, useEffect, useRef } from 'react';
import { useSystemVitality } from './useSystemVitality';

export type AvatarState = 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';

export const useAvatarState = () => {
    const { lastEvent } = useSystemVitality();
    const [state, setState] = useState<AvatarState>('idle');
    const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    useEffect(() => {
        if (!lastEvent) return;

        const { type } = lastEvent;

        const transitionTo = (newState: AvatarState, duration: number = 3000) => {
            // Clear existing timer to prevent state thrashing
            if (timerRef.current) clearTimeout(timerRef.current);

            setState(newState);

            // Return to idle after duration
            timerRef.current = setTimeout(() => {
                setState('idle');
            }, duration);
        };

        switch (type) {
            case 'level_up':
                transitionTo('awakened', 5000);
                break;
            case 'job_started':
                transitionTo('thinking', 0); // Thinking stays until stopped or timeout
                break;
            case 'job_completed':
                setState('idle');
                break;
            case 'inspiration':
                transitionTo('meditating', 4000);
                break;
            case 'tts_started':
                transitionTo('speaking', 0);
                break;
            case 'tts_completed':
                setState('idle');
                break;
            case 'skill_loaded':
                transitionTo('learning', 4000);
                break;
            default:
                // For karma_update, maybe a small reaction?
                break;
        }
    }, [lastEvent]);

    useEffect(() => {
        return () => {
            if (timerRef.current) clearTimeout(timerRef.current);
        };
    }, []);

    return state;
};
