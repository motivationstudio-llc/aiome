use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

/// システム全体の稼働状況 (Heartbeat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHeartbeat {
    pub cpu_usage: f32,
    pub memory_usage_mb: u64,
    pub vram_usage_mb: u64, // Mock value for M4 Pro
    pub active_actor: Option<String>,
}

/// ログイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// テレメトリ配信局 (TelemetryHub)
/// 
/// 複数の WebSocket クライアントに対して、1対多で情報をブロードキャストする。
pub struct TelemetryHub {
    tx_heartbeat: broadcast::Sender<SystemHeartbeat>,
    tx_log: broadcast::Sender<LogEvent>,
    system: Arc<Mutex<System>>,
}

impl TelemetryHub {
    pub fn new() -> Self {
        let (tx_hb, _) = broadcast::channel(16);
        let (tx_lg, _) = broadcast::channel(100);
        
        // sysinfo v0.30+ initialization
        let r = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything());
        let sys = System::new_with_specifics(r);

        Self {
            tx_heartbeat: tx_hb,
            tx_log: tx_lg,
            system: Arc::new(Mutex::new(sys)),
        }
    }

    pub fn subscribe_heartbeat(&self) -> broadcast::Receiver<SystemHeartbeat> {
        self.tx_heartbeat.subscribe()
    }

    pub fn subscribe_log(&self) -> broadcast::Receiver<LogEvent> {
        self.tx_log.subscribe()
    }

    pub fn broadcast_log(&self, level: &str, message: &str) {
        let event = LogEvent {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        };
        // 誰も聞いていなければ無視
        let _ = self.tx_log.send(event); 
    }

    /// 現在のシステム状態を即座に計測して返す (Real-Time Interoception用)
    pub fn get_current_status(&self) -> SystemHeartbeat {
        let (cpu, mem) = {
            let mut s = self.system.lock().unwrap();
            s.refresh_cpu();
            s.refresh_memory();
            (s.global_cpu_info().cpu_usage(), s.used_memory() / 1024 / 1024)
        };

        SystemHeartbeat {
            cpu_usage: cpu,
            memory_usage_mb: mem,
            vram_usage_mb: mem / 2, // M4 Pro Unified Memory Mock
            active_actor: None,
        }
    }

    /// 定期的にシステムリソースを計測して配信する
    pub async fn start_heartbeat_loop(&self) {
        let tx = self.tx_heartbeat.clone();
        let sys = self.system.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            loop {
                interval.tick().await;
                
                let (cpu, mem) = {
                    let mut s = sys.lock().unwrap();
                    s.refresh_cpu();
                    s.refresh_memory();
                    (s.global_cpu_info().cpu_usage(), s.used_memory() / 1024 / 1024)
                };

                // M4 Pro Unified Memory Mock
                let vram_mock = mem / 2; 

                let hb = SystemHeartbeat {
                    cpu_usage: cpu,
                    memory_usage_mb: mem,
                    vram_usage_mb: vram_mock,
                    active_actor: None, 
                };

                // Receiver がいない場合はエラーになるが無視
                let _ = tx.send(hb);
            }
        });
    }
}
