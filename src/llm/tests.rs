use super::*;

#[test]
fn truncate_marks_elision() {
    assert_eq!(truncate("abcdef", 3), "abc…");
    assert_eq!(truncate("abc", 3), "abc");
}

#[test]
fn detail_prefers_error_message_field() {
    let body = r#"{"error":{"message":"quota exceeded"}}"#;
    assert_eq!(detail(429, body), "HTTP 429: quota exceeded");
}

#[test]
fn detail_falls_back_to_raw_body() {
    assert_eq!(detail(500, "boom"), "HTTP 500: boom");
    assert_eq!(detail(500, ""), "HTTP 500");
}

#[test]
fn client_requires_a_model() {
    let config = Config::default();
    assert!(Client::from_config(&config).is_err());
}

#[test]
fn client_from_config_uses_timeout_and_model() {
    let mut config = Config::default();
    config.ai.model = "gemini-3-flash".into();
    let client = Client::from_config(&config).unwrap();
    assert_eq!(
        client.url("chat/completions"),
        format!("{}chat/completions", config.ai.base_url)
    );
}
