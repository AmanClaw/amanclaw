/// Trait for emitting events from the engine to external observers (gateway, monitoring).
pub trait EventEmitter: Send + Sync {
    fn emit(&self, topic: &str, data: serde_json::Value);
}

/// No-op emitter for CLI mode — zero overhead.
pub struct NoopEmitter;

impl EventEmitter for NoopEmitter {
    fn emit(&self, _topic: &str, _data: serde_json::Value) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_emitter() {
        let emitter = NoopEmitter;
        // Should not panic
        emitter.emit("test.event", serde_json::json!({"key": "value"}));
    }
}
