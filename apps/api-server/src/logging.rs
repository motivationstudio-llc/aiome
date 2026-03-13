use serde::Serialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{Event, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

pub struct DbLoggerLayer {
    tx: mpsc::Sender<LogEntry>,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub target: String,
    pub message: String,
}

impl DbLoggerLayer {
    pub fn new(pool: SqlitePool) -> Self {
        let (tx, mut rx) = mpsc::channel::<LogEntry>(1000);

        tokio::spawn(async move {
            // Ensure table exists
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS app_logs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                    level TEXT NOT NULL,
                    target TEXT NOT NULL,
                    message TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await;

            while let Some(entry) = rx.recv().await {
                // Ignore inserts if queue is too large or db fails (silent drop for logging layer)
                let _ =
                    sqlx::query("INSERT INTO app_logs (level, target, message) VALUES (?, ?, ?)")
                        .bind(entry.level)
                        .bind(entry.target)
                        .bind(entry.message)
                        .execute(&pool)
                        .await;
            }
        });

        Self { tx }
    }
}

impl<S: Subscriber> Layer<S> for DbLoggerLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level().to_string();
        let target = event.metadata().target().to_string();

        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let entry = LogEntry {
            level,
            target,
            message: visitor.message,
        };

        // Fire and forget (don't block the actual thread emitting log)
        let _ = self.tx.try_send(entry);
    }
}

struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // remove surrounding quotes if any
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        }
    }
}
