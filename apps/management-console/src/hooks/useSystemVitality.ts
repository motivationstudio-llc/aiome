import { useEffect, useState, useCallback, useRef } from 'react';
import { API_BASE } from '../config';
import { AgentStats, Karma } from '../types';
import { getAuthHeaders } from '../lib/auth';
import { fetchEventSource } from '@microsoft/fetch-event-source';

export interface SystemVitality {
    status: 'idle' | 'thinking' | 'speaking' | 'learning' | 'meditating' | 'awakened';
    data: AgentStats | Karma | unknown;
}

export type VitalityEvent = {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready' | 'immune_alert' | 'skill_execution' | 'agent_stats' | 'proactive_talk';
    data: AgentStats | Karma | unknown;
};

type ConnectionStatus = 'connected' | 'connecting' | 'disconnected' | 'paused';

export const useSystemVitality = () => {
    const [events, setEvents] = useState<VitalityEvent[]>([]);
    const [lastEvent, setLastEvent] = useState<VitalityEvent | null>(null);
    const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('connecting');
    // We keep track of whether the user intentionally paused the connection
    const [isPaused, setIsPaused] = useState(false);
    // Add a counter to force useEffect to re-run and reconnect immediately 
    const [retryTrigger, setRetryTrigger] = useState(0);

    const abortControllerRef = useRef<AbortController | null>(null);

    const addEvent = useCallback((type: VitalityEvent['type'], data: VitalityEvent['data']) => {
        const newEvent = { type, data };
        setEvents(prev => [newEvent, ...prev].slice(0, 50));
        setLastEvent(newEvent);
    }, []);

    const toggleConnection = useCallback(() => {
        if (connectionStatus === 'disconnected') {
            // If disconnected, clicking it forces an immediate reconnect attempt
            setRetryTrigger(prev => prev + 1);
            setIsPaused(false);
        } else {
            // If connected/paused, toggle the pause state
            setIsPaused(prev => !prev);
        }
    }, [connectionStatus]);

    useEffect(() => {
        if (isPaused) {
            setConnectionStatus('paused');
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
                abortControllerRef.current = null;
            }
            return; // Don't attempt to connect if intentionally paused
        }

        let retryCount = 0;
        let timeoutId: any = null;

        const connect = async () => {
            if (timeoutId) clearTimeout(timeoutId);
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
            }

            abortControllerRef.current = new AbortController();
            setConnectionStatus('connecting');

            try {
                await fetchEventSource(`${API_BASE}/api/system/vitality`, {
                    method: 'GET',
                    headers: {
                        ...getAuthHeaders(),
                        'Accept': 'text/event-stream'
                    },
                    signal: abortControllerRef.current.signal,
                    onopen: async (response) => {
                        if (response.ok) {
                            console.log("✨ [SSE] Connection established via custom fetch");
                            setConnectionStatus('connected');
                            retryCount = 0;
                            return; // everything is fine
                        }
                        throw new Error(`Failed to connect to SSE: ${response.status}`);
                    },
                    onmessage: (msg) => {
                        if (!msg.event || !msg.data) return;

                        // We check if the event is one of our mapped ones
                        const validEvents = [
                            'level_up', 'karma_update', 'inspiration',
                            'job_started', 'job_completed',
                            'tts_started', 'tts_completed',
                            'skill_loaded', 'skill_ready',
                            'immune_alert', 'skill_execution', 'agent_stats', 'proactive_talk'
                        ];

                        if (validEvents.includes(msg.event)) {
                            try {
                                const data = msg.data ? JSON.parse(msg.data) : null;
                                addEvent(msg.event as VitalityEvent['type'], data);
                            } catch (err) {
                                console.error(`Error parsing SSE event ${msg.event}:`, err);
                            }
                        }
                    },
                    onclose: () => {
                        console.warn("⚠️ [SSE] Connection closed from server, retrying...");
                        setConnectionStatus('disconnected');
                        throw new Error("Connection closed"); // Trigger error to enter retry logic
                    },
                    onerror: (err) => {
                        console.error("⚠️ [SSE] Connection Error:", err);
                        setConnectionStatus('disconnected');
                        // Calculate exponential backoff
                        const delay = Math.min(1000 * Math.pow(2, retryCount), 10000);
                        retryCount++;

                        // Only retry if not aborted intentionally
                        if (abortControllerRef.current && !abortControllerRef.current.signal.aborted) {
                            timeoutId = setTimeout(connect, delay);
                        }

                        // Prevent fetchEventSource from running its own internal retry loop
                        throw err;
                    }
                });
            } catch (err) {
                // Handled in onerror or abort
            }
        };

        connect();

        return () => {
            if (abortControllerRef.current) {
                abortControllerRef.current.abort();
                abortControllerRef.current = null;
            }
            if (timeoutId) clearTimeout(timeoutId);
        };
    }, [addEvent, isPaused, retryTrigger]);

    return { events, lastEvent, connectionStatus, toggleConnection };
};
