import { useEffect, useState } from 'react';
import { ExternalLink, RotateCw, Package } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export interface ProjectSummary {
    id: string;
    title: string;
    style: string | null;
    created_at: string;
    preview_url: string | null;
}

export function Warehouse({ onRemix }: { onRemix: (project: ProjectSummary) => void }) {
    const [projects, setProjects] = useState<ProjectSummary[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        invoke<ProjectSummary[]>('get_projects')
            .then(data => {
                setProjects(data);
                setLoading(false);
            })
            .catch(err => {
                console.error("Failed to load projects:", err);
                setError(typeof err === 'string' ? err : 'Failed to connect to Core');
                setLoading(false);
            });
    }, []);

    if (loading) {
        return (
            <div className="h-full flex items-center justify-center text-sonar-green animate-pulse font-mono">
                SCANNING DATABASE...
            </div>
        );
    }

    if (error) {
        return (
            <div className="h-full flex flex-col items-center justify-center text-sonar-red font-mono gap-4">
                <div className="text-2xl">⚠️ CORE OFFLINE</div>
                <div className="text-sm text-gray-500">{error}</div>
                <button
                    onClick={() => { setError(null); setLoading(true); invoke<ProjectSummary[]>('get_projects').then(d => { setProjects(d); setLoading(false); }).catch(e => { setError(typeof e === 'string' ? e : 'Connection failed'); setLoading(false); }); }}
                    className="px-4 py-2 border border-sonar-green text-sonar-green hover:bg-sonar-green hover:text-black transition-all rounded"
                >
                    RETRY
                </button>
            </div>
        );
    }

    return (
        <div className="h-full p-8 overflow-y-auto custom-scrollbar">
            <header className="mb-8 flex items-end justify-between border-b border-gray-800 pb-4">
                <div>
                    <h1 className="text-3xl font-light text-white tracking-tight">
                        THE <span className="text-sonar-green font-bold">WAREHOUSE</span>
                    </h1>
                    <p className="text-gray-500 text-sm mt-1 font-mono">
                        ARCHIVE SIZE: {projects.length} UNITS
                    </p>
                </div>
            </header>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
                {projects.map((project) => (
                    <div
                        key={project.id}
                        className="group relative bg-gray-900/50 border border-gray-800 hover:border-sonar-green/50 rounded-xl overflow-hidden transition-all duration-300 hover:shadow-[0_0_20px_rgba(0,255,65,0.1)]"
                    >
                        {/* Resource Preview Area */}
                        <div className="aspect-video bg-black relative">
                            {project.preview_url ? (
                                project.preview_url.match(/\.(mp4|webm)$/i) ? (
                                    <video
                                        src={`http://localhost:3000${project.preview_url}`}
                                        className="w-full h-full object-cover opacity-60 group-hover:opacity-100 transition-opacity"
                                        muted
                                        loop
                                        onMouseOver={e => e.currentTarget.play()}
                                        onMouseOut={e => {
                                            e.currentTarget.pause();
                                            e.currentTarget.currentTime = 0;
                                        }}
                                    />
                                ) : (
                                    <img
                                        src={`http://localhost:3000${project.preview_url}`}
                                        alt={project.title}
                                        className="w-full h-full object-cover opacity-60 group-hover:opacity-100 transition-opacity"
                                    />
                                )
                            ) : (
                                <div className="w-full h-full flex items-center justify-center text-gray-700">
                                    <Package size={48} strokeWidth={1} />
                                </div>
                            )}

                            {/* Overlay Actions */}
                            <div className="absolute inset-0 bg-black/60 flex items-center justify-center gap-4 opacity-0 group-hover:opacity-100 transition-opacity backdrop-blur-sm">
                                <button className="p-3 bg-white text-black rounded-full hover:bg-sonar-green hover:scale-110 transition-all">
                                    <ExternalLink size={24} />
                                </button>
                                <button
                                    onClick={() => onRemix(project)}
                                    className="p-3 bg-gray-800 text-white rounded-full hover:bg-sonar-green hover:text-black hover:scale-110 transition-all border border-gray-600 hover:border-sonar-green"
                                >
                                    <RotateCw size={24} />
                                </button>
                            </div>
                        </div>

                        {/* Info Area */}
                        <div className="p-4 border-t border-gray-800 group-hover:border-sonar-green/30 transition-colors bg-black/40">
                            <div className="flex justify-between items-start mb-2">
                                <h3 className="font-medium text-gray-200 truncate pr-2" title={project.title}>
                                    {project.title}
                                </h3>
                                {project.style && (
                                    <span className="text-[10px] uppercase px-1.5 py-0.5 rounded bg-gray-800 text-gray-400 border border-gray-700">
                                        {project.style}
                                    </span>
                                )}
                            </div>
                            <div className="flex justify-between items-end">
                                <span className="text-xs text-sonar-green font-mono">
                                    REF: {project.id.substring(0, 6)}
                                </span>
                                <span className="text-xs text-gray-600 font-mono">
                                    {new Date(project.created_at).toLocaleDateString()}
                                </span>
                            </div>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
