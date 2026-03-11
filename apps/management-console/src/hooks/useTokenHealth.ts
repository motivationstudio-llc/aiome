import { useState, useEffect, useCallback, useRef } from 'react';
import { API_BASE } from '../config';
import { getAuthHeaders } from '../lib/auth';

interface TokenHealthState {
    isExpired: boolean;
    lastChecked: number | null;
}

/**
 * Token 健全性監視フック
 * 5分間隔で /api/health をポーリングし、401 + X-Token-Expired ヘッダーを検知する。
 */
export const useTokenHealth = (intervalMs: number = 5 * 60 * 1000) => {
    const [state, setState] = useState<TokenHealthState>({ isExpired: false, lastChecked: null });
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

    const checkHealth = useCallback(async () => {
        try {
            const res = await fetch(`${API_BASE}/api/health`, {
                headers: getAuthHeaders(),
            });

            if (res.status === 401) {
                const expired = res.headers.get('X-Token-Expired') === 'true';
                setState({ isExpired: expired, lastChecked: Date.now() });
            } else {
                setState({ isExpired: false, lastChecked: Date.now() });
            }
        } catch {
            // Network error — don't mark as expired
            setState(prev => ({ ...prev, lastChecked: Date.now() }));
        }
    }, []);

    const dismiss = useCallback(() => {
        setState(prev => ({ ...prev, isExpired: false }));
    }, []);

    useEffect(() => {
        // Initial check after short delay
        const timeout = setTimeout(checkHealth, 3000);
        timerRef.current = setInterval(checkHealth, intervalMs);

        return () => {
            clearTimeout(timeout);
            if (timerRef.current) clearInterval(timerRef.current);
        };
    }, [checkHealth, intervalMs]);

    return { ...state, checkHealth, dismiss };
};
