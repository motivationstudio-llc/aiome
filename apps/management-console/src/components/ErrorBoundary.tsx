import { Component, ErrorInfo, ReactNode } from "react";
import { ShieldAlert, RefreshCw, Home } from "lucide-react";
import { motion } from "framer-motion";

interface Props {
  children?: ReactNode;
}

interface State {
  hasError: boolean;
  error?: Error;
}

class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error:", error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      return (
        <div style={{
          height: '100vh',
          width: '100vw',
          background: 'linear-gradient(135deg, #0a0a0f 0%, #1a1a2e 100%)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: '#fff',
          fontFamily: 'system-ui, -apple-system, sans-serif'
        }}>
          <motion.div
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            style={{
              maxWidth: '500px',
              padding: '3rem',
              background: 'rgba(255, 255, 255, 0.03)',
              borderRadius: '24px',
              border: '1px solid rgba(255, 77, 109, 0.2)',
              textAlign: 'center',
              backdropFilter: 'blur(20px)',
              boxShadow: '0 20px 50px rgba(0,0,0,0.3)'
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'center', marginBottom: '1.5rem' }}>
              <div style={{ 
                width: '64px', height: '64px', borderRadius: '50%', background: 'rgba(255, 77, 109, 0.1)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', border: '1px solid rgba(255, 77, 109, 0.3)'
              }}>
                <ShieldAlert color="#ff4d6d" size={32} />
              </div>
            </div>

            <h1 style={{ fontSize: '1.8rem', fontWeight: 800, marginBottom: '1rem', letterSpacing: '-0.02em' }}>
              Neural Sync Interrupted
            </h1>
            <p style={{ color: 'rgba(255,255,255,0.6)', marginBottom: '2rem', fontSize: '0.95rem', lineHeight: 1.6 }}>
              A fatal exception occurred in the neural interface. The system has initiated protective isolation to preserve data integrity.
            </p>

            <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
              <button
                onClick={() => window.location.reload()}
                style={{
                  padding: '0.8rem 1.5rem', borderRadius: '12px', background: 'var(--accent-cyan, #00f2ff)',
                  border: 'none', color: '#000', fontWeight: 700, display: 'flex', alignItems: 'center', gap: '0.5rem',
                  cursor: 'pointer'
                }}
              >
                <RefreshCw size={18} />
                Re-initialize
              </button>
              <button
                onClick={() => window.location.href = '/'}
                style={{
                  padding: '0.8rem 1.5rem', borderRadius: '12px', background: 'rgba(255,255,255,0.05)',
                  border: '1px solid rgba(255,255,255,0.1)', color: '#fff', fontWeight: 600, display: 'flex', alignItems: 'center', gap: '0.5rem',
                  cursor: 'pointer'
                }}
              >
                <Home size={18} />
                Home
              </button>
            </div>
            
            {this.state.error && (
              <div style={{ marginTop: '2rem', padding: '1rem', background: 'rgba(0,0,0,0.3)', borderRadius: '12px', textAlign: 'left', fontSize: '0.75rem', overflow: 'auto', maxHeight: '100px', color: 'rgba(255,255,255,0.4)', border: '1px solid rgba(255,255,255,0.05)' }}>
                <code>{this.state.error.toString()}</code>
              </div>
            )}
          </motion.div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
