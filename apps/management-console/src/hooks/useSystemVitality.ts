import { useEffect, useState, useCallback } from 'react';
import { API_BASE } from '../config';

export type VitalityEvent = {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready';
    data: any;
};

export const useSystemVitality = () => {
    const [events, setEvents] = useState<VitalityEvent[]>([]);
    const [lastEvent, setLastEvent] = useState<VitalityEvent | null>(null);

    const addEvent = useCallback((type: VitalityEvent['type'], data: any) => {
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
            'skill_loaded', 'skill_ready'
        ];

        eventNames.forEach(name => {
            eventSource.addEventListener(name, (e: any) => {
                try {
                    const data = e.data ? JSON.parse(e.data) : null;
                    addEvent(name, data);
                } catch (err) {
                    console.error(`Error parsing SSE event ${name}:`, err);
                }
            });
        });

        return () => eventSource.close();
    }, [addEvent]);

    return { events, lastEvent };
};
