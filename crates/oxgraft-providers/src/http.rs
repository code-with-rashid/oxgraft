//! Minimal blocking JSON-over-HTTP with error classification (SPEC-0001 AC5).

use std::time::Duration;

use oxgraft_seams::{ErrorKind, ProviderError};
use serde_json::Value;

pub(crate) fn post_json(
    provider: &str,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
    timeout: Duration,
) -> Result<Value, ProviderError> {
    send(provider, url, headers, body, timeout)?
        .into_json::<Value>()
        .map_err(|e| ProviderError::new(provider, ErrorKind::Protocol, format!("response was not valid JSON: {e}")))
}

/// POST and read a `text/event-stream` response, calling `on_event(event, data)` for
/// each server-sent event. A connection that drops mid-stream is a transient error.
pub(crate) fn post_sse(
    provider: &str,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
    timeout: Duration,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), ProviderError>,
) -> Result<(), ProviderError> {
    let response = send(provider, url, headers, body, timeout)?;
    read_sse(provider, std::io::BufReader::new(response.into_reader()), on_event)
}

/// Parse server-sent events from `reader` (one `event:` and `data:` lines per event,
/// blank-line separated; multi-line data joined with `\n`; comments ignored).
pub(crate) fn read_sse(
    provider: &str,
    reader: impl std::io::BufRead,
    on_event: &mut dyn FnMut(&str, &str) -> Result<(), ProviderError>,
) -> Result<(), ProviderError> {
    let (mut event, mut data) = (String::new(), String::new());
    for line in reader.lines() {
        let line =
            line.map_err(|e| ProviderError::new(provider, ErrorKind::Transient, format!("stream interrupted: {e}")))?;
        let line = line.strip_suffix('\r').unwrap_or(&line);
        if line.is_empty() {
            if !data.is_empty() || !event.is_empty() {
                on_event(&event, &data)?;
            }
            event.clear();
            data.clear();
        } else if let Some(v) = line.strip_prefix("event:") {
            event = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(v.strip_prefix(' ').unwrap_or(v));
        }
    }
    if !data.is_empty() {
        on_event(&event, &data)?;
    }
    Ok(())
}

fn send(
    provider: &str,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
    timeout: Duration,
) -> Result<ureq::Response, ProviderError> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(30)).timeout(timeout).build();
    let mut request = agent.post(url);
    for (name, value) in headers {
        request = request.set(name, value);
    }
    match request.send_json(body) {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(code, response)) => {
            let retry_after_ms = response
                .header("retry-after")
                .and_then(|v| v.trim().parse::<f64>().ok())
                .map(|secs| (secs.max(0.0) * 1000.0) as u64);
            let should_retry = response.header("x-should-retry").map(|v| v.trim().to_ascii_lowercase());
            let text = response.into_string().unwrap_or_default();
            let mut kind = classify(code);
            match should_retry.as_deref() {
                Some("false") => {
                    if kind == ErrorKind::Transient {
                        kind = ErrorKind::BadRequest;
                    }
                }
                Some("true") => kind = ErrorKind::Transient,
                _ => {}
            }
            Err(ProviderError {
                provider: provider.into(),
                kind,
                status: Some(code),
                message: format!("HTTP {code}: {}", error_message(&text)),
                retry_after_ms,
            })
        }
        Err(ureq::Error::Transport(t)) => {
            Err(ProviderError::new(provider, ErrorKind::Transient, format!("network error calling {url}: {t}")))
        }
    }
}

pub(crate) fn classify(status: u16) -> ErrorKind {
    match status {
        401 | 403 => ErrorKind::Auth,
        408 | 409 | 425 | 429 => ErrorKind::Transient,
        500..=599 => ErrorKind::Transient,
        _ => ErrorKind::BadRequest,
    }
}

/// Pull a human-readable message out of common error body shapes.
pub(crate) fn error_message(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        for ptr in ["/error/message", "/message", "/error", "/detail"] {
            if let Some(s) = v.pointer(ptr).and_then(Value::as_str) {
                return s.to_string();
            }
        }
    }
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "(empty response body)".into();
    }
    trimmed.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPEC-0001 AC5
    #[test]
    fn classifies_statuses() {
        assert_eq!(classify(429), ErrorKind::Transient);
        assert_eq!(classify(529), ErrorKind::Transient);
        assert_eq!(classify(500), ErrorKind::Transient);
        assert_eq!(classify(401), ErrorKind::Auth);
        assert_eq!(classify(400), ErrorKind::BadRequest);
        assert_eq!(classify(404), ErrorKind::BadRequest);
    }

    // SPEC-0001 AC8
    #[test]
    fn parses_server_sent_events() {
        let text = ": comment\r\nevent: a\r\ndata: {\"x\":1}\r\n\r\ndata: line1\ndata: line2\n\nevent: b\ndata: tail";
        let mut seen = Vec::new();
        read_sse("t", text.as_bytes(), &mut |e, d| {
            seen.push((e.to_string(), d.to_string()));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            vec![
                ("a".to_string(), "{\"x\":1}".to_string()),
                (String::new(), "line1\nline2".to_string()),
                ("b".to_string(), "tail".to_string())
            ]
        );
    }

    #[test]
    fn extracts_messages_from_known_shapes() {
        assert_eq!(error_message(r#"{"type":"error","error":{"type":"x","message":"bad model"}}"#), "bad model");
        assert_eq!(error_message(r#"{"error":"nope"}"#), "nope");
        assert_eq!(error_message("  "), "(empty response body)");
        assert_eq!(error_message("plain"), "plain");
    }
}
