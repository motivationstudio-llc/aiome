import React, { useEffect, useRef, useState } from 'react';
import { Network } from "vis-network";
import { DataSet } from "vis-data";
import { GitMerge, ZoomIn, ZoomOut, RefreshCw, Layers } from 'lucide-react';
import { API_BASE } from "../config";

const GraphView: React.FC = () => {
    const containerRef = useRef<HTMLDivElement>(null);
    const networkRef = useRef<Network | null>(null);
    const [nodeCount, setNodeCount] = useState(0);

    useEffect(() => {
        if (!containerRef.current) return;

        const initGraph = async () => {
            try {
                const res = await fetch(`${API_BASE}/api/synergy/graph`);
                const data = await res.json();

                const nodes = new DataSet(data.nodes.map((n: any) => ({
                    ...n,
                    color: {
                        background: n.id.startsWith('job_') ? '#00f2ff22' : '#bc8cff22',
                        border: n.id.startsWith('job_') ? 'var(--accent-cyan)' : 'var(--accent-purple)',
                        highlight: {
                            background: n.id.startsWith('job_') ? '#00f2ff44' : '#bc8cff44',
                            border: n.id.startsWith('job_') ? '#fff' : '#fff',
                        }
                    },
                    font: { color: '#fff', size: 12, face: 'Inter' },
                    shape: 'dot',
                    size: 20 + (n.label.length / 5)
                })));

                const edges = new DataSet(data.edges.map((e: any) => ({
                    ...e,
                    color: { color: 'rgba(255,255,255,0.1)', highlight: 'var(--accent-cyan)' },
                    width: 1,
                    smooth: { type: 'continuous' }
                })));

                setNodeCount(nodes.length);

                const options = {
                    nodes: {
                        borderWidth: 2,
                        shadow: { enabled: true, color: 'rgba(0,0,0,0.5)', size: 10, x: 5, y: 5 }
                    },
                    edges: { arrows: 'to' },
                    physics: {
                        stabilization: true,
                        barnesHut: {
                            gravitationalConstant: -2000,
                            centralGravity: 0.3,
                            springLength: 95,
                            springConstant: 0.04,
                            damping: 0.09,
                            avoidOverlap: 0.1
                        }
                    },
                    interaction: {
                        hover: true,
                        tooltipDelay: 200,
                        zoomView: true
                    }
                };

                networkRef.current = new Network(containerRef.current!, { nodes: nodes as any, edges: edges as any }, options);
            } catch (e) {
                console.error("Graph failed to load", e);
            }
        };

        initGraph();

        return () => {
            networkRef.current?.destroy();
        };
    }, []);

    const zoomIn = () => networkRef.current?.moveTo({ scale: (networkRef.current.getScale() * 1.2) });
    const zoomOut = () => networkRef.current?.moveTo({ scale: (networkRef.current.getScale() / 1.2) });
    const fit = () => networkRef.current?.fit();

    return (
        <div className="main-panel ani-fade" style={{ height: '78vh', display: 'flex', flexDirection: 'column', padding: 0, position: 'relative' }}>
            <div className="panel-header" style={{ padding: '1rem 1.5rem', borderBottom: '1px solid var(--border-glass)', zIndex: 10 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <GitMerge size={20} color="var(--accent-cyan)" />
                    <h3>Synapse Resonance Map</h3>
                </div>
                <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)' }}>{nodeCount} NODES CONNECTED</div>
                    <button className="nav-item" style={{ margin: 0, padding: '0.4rem 0.75rem' }} onClick={fit}>
                        <RefreshCw size={14} /> RE-CENTER
                    </button>
                </div>
            </div>

            <div ref={containerRef} style={{ flex: 1, background: 'radial-gradient(circle at center, #0d1117 0%, #050505 100%)' }} />

            {/* Overlay Controls */}
            <div style={{ position: 'absolute', right: '1.5rem', bottom: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem', zIndex: 10 }}>
                <button
                    onClick={zoomIn}
                    style={{ width: '40px', height: '40px', background: 'var(--bg-glass-heavy)', border: '1px solid var(--border-glass)', borderRadius: '8px', color: '#fff', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <ZoomIn size={18} />
                </button>
                <button
                    onClick={zoomOut}
                    style={{ width: '40px', height: '40px', background: 'var(--bg-glass-heavy)', border: '1px solid var(--border-glass)', borderRadius: '8px', color: '#fff', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <ZoomOut size={18} />
                </button>
                <button
                    style={{ width: '40px', height: '40px', background: 'var(--bg-glass-heavy)', border: '1px solid var(--border-glass)', borderRadius: '8px', color: '#fff', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                >
                    <Layers size={18} />
                </button>
            </div>

            {/* Hint */}
            <div style={{ position: 'absolute', left: '1.5rem', bottom: '1.5rem', background: 'rgba(0,0,0,0.5)', padding: '0.5rem 1rem', borderRadius: '8px', border: '1px solid var(--border-glass)', fontSize: '0.75rem', color: 'var(--text-muted)', zIndex: 10 }}>
                Drag to pan • Scroll to zoom • Click nodes to focus
            </div>
        </div>
    );
};

export default GraphView;
