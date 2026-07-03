use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};

#[derive(Clone)]
pub struct EventSender {
    ts_fn: ThreadsafeFunction<String>,
}

impl EventSender {
    pub fn new(ts_fn: ThreadsafeFunction<String>) -> Self {
        Self { ts_fn }
    }

    pub fn send(&self, event_type: &str, payload: serde_json::Value) {
        let mut full_payload = payload.clone();
        if let serde_json::Value::Object(ref mut map) = full_payload {
            map.insert("type".to_string(), serde_json::Value::String(event_type.to_string()));
        } else {
            full_payload = serde_json::json!({
                "type": event_type,
                "data": payload
            });
        }
        if let Ok(json_str) = serde_json::to_string(&full_payload) {
            let _ = self.ts_fn.call(Ok(json_str), ThreadsafeFunctionCallMode::NonBlocking);
        }
    }
}
