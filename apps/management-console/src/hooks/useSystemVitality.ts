import { useEffect, useState, useCallback } from 'react';
import { API_BASE } from '../config';

import { AgentStats, Karma } from '../types';

export interface SystemVitality {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    data: AgentStats | Karma | unknown;
}

export type VitalityEvent = {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready' | 'immune_alert' | 'skill_execution';
    data: AgentStats | Karma | unknown;
};

export const useSystemVitality = () => {
    const [events, setEvents] = useState<VitalityEvent[]>([]);
    const [lastEvent, setLastEvent] = useState<VitalityEvent | null>(null);

    const addEvent = useCallback((type: VitalityEvent['type'], data: VitalityEvent['data']) => {
        const newEvent = { type, data };
        setEvents(prev => [newEvent, ...prev].slice(0, 50));
        setLastEvent(newEvent);
    }, []);

    useEffect(() => {
        const eventSource = new EventSource(`${API_BASE}/api/system/vitality`);

        eventSource.onmessage = () => {
            // Handle generic message if needed
        };

        const eventNames: VitalityEvent['type'][] = [
            'level_up', 'karma_update', 'inspiration',
            'job_started', 'job_completed',
            'tts_started', 'tts_completed',
            'skill_loaded', 'skill_ready',
            'immune_alert', 'skill_execution'
        ];

        const bindEvent = (name: VitalityEvent['type']) => {
            eventSource.addEventListener(name, (e: MessageEvent) => {
                try {
                    const data = e.data ? JSON.parse(e.data) : null;
                    addEvent(name, data);
                } catch (err) {
                    console.error(`Error parsing SSE event ${name}:`, err);
                }
            });
        };

        eventNames.forEach(name => {
            bindEvent(name);
        });

        eventSource.onerror = (err) => {
            console.error("SSE Connection Error:", err);
            eventSource.close();
        };

        return () => eventSource.close();
    }, [addEvent]);

    return { events, lastEvent };
};
