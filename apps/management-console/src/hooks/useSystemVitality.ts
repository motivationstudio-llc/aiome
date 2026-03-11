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
        let eventSource: EventSource | null = null;
        let retryCount = 0;
        let timeoutId: any = null;

        const connect = () => {
            if (timeoutId) clearTimeout(timeoutId);
            if (eventSource) eventSource.close();

            eventSource = new EventSource(`${API_BASE}/api/system/vitality`);

            const eventNames: VitalityEvent['type'][] = [
                'level_up', 'karma_update', 'inspiration',
                'job_started', 'job_completed',
                'tts_started', 'tts_completed',
                'skill_loaded', 'skill_ready',
                'immune_alert', 'skill_execution'
            ];

            eventNames.forEach(name => {
                eventSource!.addEventListener(name, (e: MessageEvent) => {
                    try {
                        const data = e.data ? JSON.parse(e.data) : null;
                        addEvent(name, data);
                    } catch (err) {
                        console.error(`Error parsing SSE event ${name}:`, err);
                    }
                });
            });

            eventSource.onopen = () => {
                console.log("✨ [SSE] Connection established");
                retryCount = 0;
            };

            eventSource.onerror = (err) => {
                console.error("⚠️ [SSE] Connection Error, retrying...", err);
                eventSource?.close();

                const delay = Math.min(1000 * Math.pow(2, retryCount), 30000);
                retryCount++;

                timeoutId = setTimeout(connect, delay);
            };
        };

        connect();

        return () => {
            if (eventSource) eventSource.close();
            if (timeoutId) clearTimeout(timeoutId);
        };
    }, [addEvent]);

    return { events, lastEvent };
};
