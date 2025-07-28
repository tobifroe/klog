use rand::Rng;

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn get_rnd_color() -> Color {
    let mut rng = rand::rng();
    let r = rng.random_range(0..255);
    let g = rng.random_range(0..255);
    let b = rng.random_range(0..255);
    Color { r, g, b }
}

pub fn maybe_parse_json(line: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(line).ok()
}

pub fn get_pretty_json(value: serde_json::Value) -> String {
    let ts_keys = ["ts", "timestamp", "time"];
    let msg_keys = ["msg", "message", "log"];
    let level_keys = ["level", "lvl", "severity"];

    let ts = ts_keys
        .iter()
        .find_map(|k| value.get(k)?.as_str())
        .unwrap_or("no-ts");

    let level = level_keys
        .iter()
        .find_map(|k| value.get(k)?.as_str())
        .unwrap_or("INFO");

    let msg = msg_keys
        .iter()
        .find_map(|k| value.get(k)?.as_str())
        .unwrap_or("no-msg");

    format!("[{}] {}: {}", level, ts, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_randomness() {
        let color1 = get_rnd_color();
        let color2 = get_rnd_color();
        assert!(color1.r != color2.r || color1.g != color2.g || color1.b != color2.b);
    }

    #[test]
    fn test_maybe_parse_json_valid() {
        let line = r#"{"msg":"hello","ts":"2025-07-28T12:34:56Z"}"#;
        let result = maybe_parse_json(line);
        assert!(result.is_some());
        let json = result.unwrap();
        assert_eq!(json["msg"], "hello");
        assert_eq!(json["ts"], "2025-07-28T12:34:56Z");
    }

    #[test]
    fn test_maybe_parse_json_invalid() {
        let line = "this is not json";
        let result = maybe_parse_json(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_pretty_json_standard_fields() {
        let value = json!({
            "timestamp": "2025-07-28T12:00:00Z",
            "message": "Started up",
            "level": "info"
        });
        let result = get_pretty_json(value);
        assert_eq!(result, "[info] 2025-07-28T12:00:00Z: Started up");
    }

    #[test]
    fn test_get_pretty_json_alt_fields() {
        let value = json!({
            "ts": "2025-07-28T12:01:00Z",
            "msg": "Service healthy",
            "lvl": "debug"
        });
        let result = get_pretty_json(value);
        assert_eq!(result, "[debug] 2025-07-28T12:01:00Z: Service healthy");
    }

    #[test]
    fn test_get_pretty_json_log_field() {
        let value = json!({
            "log": "Request received",
            "time": "2025-07-28T12:02:00Z"
        });
        let result = get_pretty_json(value);
        assert_eq!(result, "[INFO] 2025-07-28T12:02:00Z: Request received");
    }

    #[test]
    fn test_get_pretty_json_missing_all_fields() {
        let value = json!({
            "foo": "bar"
        });
        let result = get_pretty_json(value);
        assert_eq!(result, "[INFO] no-ts: no-msg");
    }

    #[test]
    fn test_get_pretty_json_quoted_values_are_cleaned() {
        let value = json!({
            "ts": "2025-07-28T12:03:00Z",
            "msg": "Extra quotes",
            "level": "warn"
        });
        let result = get_pretty_json(value);
        assert_eq!(result, "[warn] 2025-07-28T12:03:00Z: Extra quotes");
    }

    #[test]
    fn test_get_pretty_json_valid() {
        let value = json!({
            "msg": "system online",
            "ts": "2025-07-28T14:00:00Z"
        });
        let output = get_pretty_json(value);
        assert_eq!(output, "[INFO] 2025-07-28T14:00:00Z: system online");
    }
}
