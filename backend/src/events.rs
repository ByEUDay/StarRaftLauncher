use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

/// 推送给 FrontEnd 的事件（经 /api/events SSE 下发）
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub kind: String,
    pub data: Value,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    pub fn emit(&self, kind: &str, data: impl Serialize) {
        let data = serde_json::to_value(data).unwrap_or(Value::Null);
        // 没有订阅者时发送失败是正常情况
        let _ = self.tx.send(Event { kind: kind.to_string(), data });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
