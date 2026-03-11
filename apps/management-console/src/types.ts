import React from 'react';

export interface AgentStats {
    level: number;
    exp: number;
    resonance: number;
    creativity: number;
    fatigue: number;
}

export interface VitalityUIEvent {
    id: number;
    title: string;
    desc: string;
    color: string;
    icon: React.ReactNode;
}

export interface VitalityRawEvent {
    type: 'level_up' | 'karma_update' | 'inspiration' | 'job_started' | 'job_completed' | 'tts_started' | 'tts_completed' | 'skill_loaded' | 'skill_ready' | 'immune_alert' | 'skill_execution';
    data: unknown;
}

export interface SystemBalance {
    id: string;
    current_health: number;
    max_health: number;
    status: string;
}

export interface GraphNode {
    id: string;
    label: string;
    group: string;
}

export interface GraphEdge {
    from: string;
    to: string;
}

export interface ImmuneRule {
    id: string;
    pattern: string;
    severity: number;
    action: string;
    created_at: string;
    risk?: string;
    active?: boolean;
}

export interface Karma {
    id: string;
    job_id: string;
    node_id: string;
    karma_type: string;
    lesson: string;
    weight: number;
    created_at: string;
}

export interface ChatMessage {
    role: 'user' | 'assistant' | 'system';
    content: string;
    isError?: boolean;
}
