import React, { useEffect, useRef, useState } from 'react';
import useWebSocket from 'react-use-websocket';
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from 'recharts';
import { Activity, Cpu, HardDrive, Terminal } from 'lucide-react';
import { clsx } from 'clsx';

interface SystemHeartbeat {
    cpu_usage: number;
    memory_usage_mb: number;
    vram_usage_mb: number;
    active_actor: string | null;
}

interface LogEvent {
    level: string;
    message: string;
    timestamp: string;
}

const WS_URL = 'ws://127.0.0.1:3000/ws';

export const AiomeLine: React.FC = () => {
    const [heartbeats, setHeartbeats] = useState<SystemHeartbeat[]>([]);
    const [logs, setLogs] = useState<LogEvent[]>([]);
    const logEndRef = useRef<HTMLDivElement>(null);

    const { lastMessage } = useWebSocket(WS_URL, {
        shouldReconnect: () => true,
        reconnectInterval: 3000,
    });

    useEffect(() => {
        if (lastMessage !== null) {
            try {
                const data = JSON.parse(lastMessage.data);
                if ('cpu_usage' in data) {
                    setHeartbeats((prev) => [...prev.slice(-20), data]);
                } else if ('level' in data) {
                    setLogs((prev) => [...prev.slice(-100), data]);
                }
            } catch (e) {
                console.error("Failed to parse WS message", e);
            }
        }
    }, [lastMessage]);

    useEffect(() => {
        logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    const currentStatus = heartbeats[heartbeats.length - 1] || { cpu_usage: 0, memory_usage_mb: 0, vram_usage_mb: 0 };

    return (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 h-full p-4 bg-sonar-black text-xs font-mono">
            {/* Resource Monitor */}
            <div className="col-span-1 bg-sonar-panel border border-gray-800 p-4 rounded-sm">
                <div className="flex items-center gap-2 mb-4 text-sonar-green">
                    <Activity size={16} />
                    <h2 className="uppercase tracking-widest font-bold">System Vitals</h2>
                </div>

                <div className="grid grid-cols-2 gap-4 mb-6">
                    <div className="bg-black/50 p-2 rounded">
                        <div className="flex items-center gap-2 text-gray-400 mb-1"><Cpu size={12} /> CPU Load</div>
                        <div className="text-xl text-white">{currentStatus.cpu_usage.toFixed(1)}%</div>
                    </div>
                    <div className="bg-black/50 p-2 rounded">
                        <div className="flex items-center gap-2 text-gray-400 mb-1"><HardDrive size={12} /> VRAM (Est)</div>
                        <div className="text-xl text-white">{currentStatus.vram_usage_mb} MB</div>
                    </div>
                </div>

                <div className="h-40 w-full">
                    <ResponsiveContainer width="100%" height="100%">
                        <LineChart data={heartbeats}>
                            <XAxis dataKey="timestamp" hide />
                            <YAxis domain={[0, 100]} hide />
                            <Tooltip
                                contentStyle={{ backgroundColor: '#0A0A0C', border: '1px solid #333' }}
                                itemStyle={{ color: '#00FF41' }}
                            />
                            <Line type="monotone" dataKey="cpu_usage" stroke="#00FF41" strokeWidth={1} dot={false} />
                            <Line type="monotone" dataKey="vram_usage_mb" stroke="#FF003C" strokeWidth={1} dot={false} />
                        </LineChart>
                    </ResponsiveContainer>
                </div>
            </div>

            {/* Terminal Log */}
            <div className="col-span-1 lg:col-span-2 bg-black border border-gray-800 p-4 rounded-sm overflow-hidden flex flex-col">
                <div className="flex items-center gap-2 mb-2 text-gray-500 border-b border-gray-900 pb-2">
                    <Terminal size={14} />
                    <span className="uppercase tracking-widest">Pipeline Log</span>
                </div>
                <div className="flex-1 overflow-y-auto font-mono text-gray-300 space-y-1">
                    {logs.map((log, i) => (
                        <div key={i} className="flex gap-2 hover:bg-white/5 p-0.5 rounded">
                            <span className="text-gray-600">[{log.timestamp}]</span>
                            <span className={clsx(
                                "font-bold w-16",
                                log.level === 'INFO' && "text-blue-400",
                                log.level === 'WARN' && "text-sonar-yellow",
                                log.level === 'ERROR' && "text-sonar-red",
                            )}>{log.level}</span>
                            <span className="break-all">{log.message}</span>
                        </div>
                    ))}
                    <div ref={logEndRef} />
                </div>
            </div>
        </div>
    );
};
