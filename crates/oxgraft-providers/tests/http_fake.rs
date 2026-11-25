//! Exercise the real HTTP paths against a local fake server (SPEC-0001 AC2-AC5).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use oxgraft_providers::anthropic::Anthropic;
use oxgraft_providers::openai::{OpenAiCompatible, PRESETS};
use oxgraft_seams::{Block, CompletionRequest, ErrorKind, Message, Provider, StopReason};
use serde_json::{Value, json};

struct Captured {
    path: String,
    headers: Vec<(String, String)>,
    body: Value,
}

/// Serve one canned response; return the base URL and a receiver for the captured request.
fn serve_once(status: u16, extra_headers: &str, body: &str) -> (String, mpsc::Receiver<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    let body = body.to_string();
    let extra = extra_headers.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
        let mut headers = Vec::new();
        let mut len = 0usize;
        loop {
            let mut h = String::new();
            reader.read_line(&mut h).unwrap();
            let h = h.trim_end().to_string();
            if h.is_empty() {
                break;
            }
            if let Some((k, v)) = h.split_once(':') {
                let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_string());
                if k == "content-length" {
                    len = v.parse().unwrap();
                }
                headers.push((k, v));
            }
        }
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).unwrap();
        let req_body: Value = serde_json::from_slice(&buf).unwrap_or(Value::Null);
        let _ = tx.send(Captured { path, headers, body: req_body });
        let reason = if status == 200 { "OK" } else { "ERR" };
        let resp = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n{extra}connection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(resp.as_bytes()).unwrap();
    });
    (format!("http://{addr}"), rx)
}

fn req(model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.into(),
        system: "be brief".into(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 100,
    }
}

fn header<'a>(c: &'a Captured, name: &str) -> Option<&'a str> {
    c.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

#[test]
fn anthropic_round_trip_over_http() {
    let (url, rx) = serve_once(
        200,
        "",
        r#"{"content":[{"type":"text","text":"hello"}],"stop_reason":"end_turn","usage":{"input_tokens":3,"output_tokens":1}}"#,
    );
    let p = Anthropic::new("test-key", &url, Some("high".into()), false);
    let c = p.complete(&req("claude-opus-5-5")).unwrap();
    assert_eq!(c.stop, StopReason::EndTurn);
    assert_eq!(c.content, vec![Block::text("hello")]);
    let cap = rx.recv().unwrap();
    assert_eq!(cap.path, "/v1/messages");
    assert_eq!(header(&cap, "x-api-key"), Some("test-key"));
    assert_eq!(header(&cap, "anthropic-version"), Some("2026-10-21"));
    assert_eq!(cap.body["system"], "be brief");
    assert_eq!(cap.body["cache_control"]["type"], "ephemeral");
}

#[test]
fn anthropic_errors_are_classified_with_retry_after() {
    let (url, _rx) = serve_once(
        429,
        "retry-after: 2\r\n",
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
    );
    let err = Anthropic::new("k", &url, None, false).complete(&req("m")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Transient);
    assert_eq!(err.retry_after_ms, Some(2000));
    assert!(err.message.contains("slow down"), "{err}");

    let (url, _rx) = serve_once(401, "", r#"{"type":"error","error":{"message":"invalid x-api-key"}}"#);
    let err = Anthropic::new("bad", &url, None, false).complete(&req("m")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Auth);

    let (url, _rx) = serve_once(500, "x-should-retry: false\r\n", "{}");
    let err = Anthropic::new("k", &url, None, false).complete(&req("m")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::BadRequest, "x-should-retry: false must win");

    let (url, _rx) = serve_once(200, "", "not json");
    let err = Anthropic::new("k", &url, None, false).complete(&req("m")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Protocol);
}

#[test]
fn connection_refused_is_transient() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let err = Anthropic::new("k", &url, None, false).complete(&req("m")).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Transient);
}

#[test]
fn openai_compatible_round_trip_over_http() {
    let (url, rx) = serve_once(
        200,
        "",
        r#"{"choices":[{"message":{"content":null,"tool_calls":[{"id":"c1","type":"function","function":{"name":"ls","arguments":"{}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#,
    );
    let preset = *PRESETS.iter().find(|p| p.id == "ollama").unwrap();
    let p = OpenAiCompatible::new(preset, &url, None);
    let c = p.complete(&req("qwen3")).unwrap();
    assert_eq!(c.stop, StopReason::ToolUse);
    assert!(matches!(&c.content[0], Block::ToolUse { name, .. } if name == "ls"));
    let cap = rx.recv().unwrap();
    assert_eq!(cap.path, "/chat/completions");
    assert_eq!(header(&cap, "authorization"), None, "local servers get no auth header");
    assert_eq!(cap.body["messages"][0]["role"], "system");

    let (url, rx) = serve_once(200, "", r#"{"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#);
    let preset = *PRESETS.iter().find(|p| p.id == "openai").unwrap();
    OpenAiCompatible::new(preset, &url, Some("sk-test".into())).complete(&req("m")).unwrap();
    let cap = rx.recv().unwrap();
    assert_eq!(header(&cap, "authorization"), Some("Bearer sk-test"));
    assert_eq!(cap.body["max_completion_tokens"], 100);
    let _ = json!(null);
}

// SPEC-0001 AC7: streaming over real HTTP yields deltas and the same completion.
#[test]
fn anthropic_streams_over_http() {
    let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"content\":[],\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\n\
event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hel\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n\
event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\n\
event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let (url, rx) = serve_once(200, "", sse);
    let p = Anthropic::new("k", &url, None, false);
    let mut deltas = Vec::new();
    let c = p.stream(&req("claude-opus-5-5"), &mut |t| deltas.push(t.to_string())).unwrap();
    assert_eq!(deltas, vec!["hel", "lo"]);
    assert_eq!(c.content, vec![Block::text("hello")]);
    assert_eq!(c.stop, StopReason::EndTurn);
    assert_eq!(rx.recv().unwrap().body["stream"], true);

    // A stream cut off before message_stop is transient (retried by core).
    let (url, _rx) = serve_once(200, "", &sse[..sse.find("event: message_delta").unwrap()]);
    let err = Anthropic::new("k", &url, None, false).stream(&req("m"), &mut |_| {}).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Transient);
}

#[test]
fn openai_compatible_streams_over_http() {
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"b\"},\"finish_reason\":\"stop\"}]}\n\n\
data: {\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n";
    let (url, rx) = serve_once(200, "", sse);
    let preset = *PRESETS.iter().find(|p| p.id == "openai").unwrap();
    let p = OpenAiCompatible::new(preset, &url, Some("k".into()));
    let mut text = String::new();
    let c = p.stream(&req("gpt-x"), &mut |t| text.push_str(t)).unwrap();
    assert_eq!(text, "ab");
    assert_eq!(c.content, vec![Block::text("ab")]);
    assert_eq!(c.usage.output_tokens, 2);
    let cap = rx.recv().unwrap();
    assert_eq!(cap.body["stream"], true);
    assert_eq!(cap.body["stream_options"]["include_usage"], true);
}
