use std::sync::Arc;

#[derive(Clone)]
pub struct EventSender {
    callback: Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>,
}

impl EventSender {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    pub fn send(&self, event_type: &str, payload: serde_json::Value) {
        let mut full_payload = payload.clone();
        if let serde_json::Value::Object(ref mut map) = full_payload {
            map.insert(
                "type".to_string(),
                serde_json::Value::String(event_type.to_string()),
            );
        } else {
            full_payload = serde_json::json!({
                "type": event_type,
                "data": payload
            });
        }
        (self.callback)(event_type, full_payload);
    }
}
