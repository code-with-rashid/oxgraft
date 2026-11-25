//! Human-readable output. Assistant text goes to stdout; activity to stderr.

use oxgraft_protocol::Event;

#[derive(Debug, Default)]
pub struct Human {
    tools: std::collections::HashMap<String, String>,
    /// Text deltas have been printed for the reply in progress (ADR 0007).
    streaming: bool,
    /// A "thinking…" line is on screen (only on a terminal).
    waiting: bool,
}

/// Model text without terminal control sequences (GRAFT-0091, after goose 1.61
/// stripping terminal controls from assistant markdown): ESC sequences, carriage
/// returns and other control characters are dropped; newlines and tabs stay.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                // CSI `ESC [ … final`, OSC `ESC ] … BEL|ST`, or a two-character escape.
                match chars.peek() {
                    Some('[') => {
                        chars.next();
                        for n in chars.by_ref() {
                            if ('@'..='~').contains(&n) {
                                break;
                            }
                        }
                    }
                    Some(']') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '\u{7}' || (n == '\u{1b}' && chars.peek() == Some(&'\\')) {
                                if n == '\u{1b}' {
                                    chars.next();
                                }
                                break;
                            }
                        }
                    }
                    Some(_) => {
                        chars.next();
                    }
                    None => {}
                }
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

fn one_line(s: &str, max: usize) -> String {
    let cleaned = plain(s);
    let line = cleaned.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    if line.chars().count() > max {
        format!("{}…", line.chars().take(max).collect::<String>())
    } else {
        line.to_string()
    }
}

impl Human {
    pub fn event(&mut self, event: &Event) {
        use std::io::{IsTerminal, Write};
        // Clear the "thinking…" line once anything else happens (GRAFT-0080).
        if self.waiting {
            self.waiting = false;
            eprint!("\r\x1b[2K");
            let _ = std::io::stderr().flush();
        }
        if let Event::TurnStarted { .. } = event {
            let ansi = !cfg!(windows) || std::env::var_os("WT_SESSION").is_some();
            if std::io::stderr().is_terminal() && ansi && std::env::var_os("NO_COLOR").is_none() {
                eprint!("\x1b[2m  … thinking\x1b[0m");
                let _ = std::io::stderr().flush();
                self.waiting = true;
            }
            return;
        }
        if self.streaming && !matches!(event, Event::TextDelta { .. }) {
            // End the streamed line before anything else is printed.
            println!();
            self.streaming = false;
            if matches!(event, Event::Message { .. }) {
                return; // already shown as it streamed
            }
        }
        match event {
            Event::TextDelta { text } => {
                if !self.streaming {
                    println!();
                    self.streaming = true;
                }
                print!("{}", plain(text));
                let _ = std::io::stdout().flush();
            }
            Event::Message { text, .. } => println!("\n{}", plain(text.trim())),
            Event::ToolCall { id, tool, input } => {
                let arg = ["path", "command", "pattern"]
                    .iter()
                    .find_map(|k| input.get(k).and_then(|v| v.as_str()))
                    .unwrap_or("");
                eprintln!("  → {tool} {}", one_line(arg, 100));
                self.tools.insert(id.clone(), tool.clone());
            }
            Event::ToolResult { ok, output, .. } => {
                if !ok {
                    eprintln!("    ✗ {}", one_line(output, 160));
                }
            }
            Event::Retrying { attempt, wait_ms, message } => {
                eprintln!("  … retry {attempt} in {:.1}s: {}", *wait_ms as f64 / 1000.0, one_line(message, 120))
            }
            Event::Warning { message } => eprintln!("  ! {message}"),
            Event::SessionFinished { ok, reason, turns, input_tokens, output_tokens } => {
                let status =
                    if *ok { "done".to_string() } else { format!("stopped: {}", reason.as_deref().unwrap_or("?")) };
                eprintln!("\n[{status} · {turns} turns · {input_tokens} in / {output_tokens} out tokens]");
                if let Some(hint) = reason.as_deref().and_then(failure_hint) {
                    eprintln!("hint: {hint}");
                }
            }
            _ => {}
        }
    }
}

/// A likely fix for a provider failure, from its message.
pub fn failure_hint(reason: &str) -> Option<&'static str> {
    let r = reason.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| r.contains(n));
    if has(&[
        "does not support tools",
        "tool use is not supported",
        "tools are not supported",
        "function calling is not",
    ]) {
        Some(
            "this model can't call tools, which Oxgraft needs; pick a tool-calling model (`oxgraft doctor --live` checks)",
        )
    } else if has(&[
        "http 401",
        "http 403",
        "invalid x-api-key",
        "invalid api key",
        "incorrect api key",
        "unauthorized",
    ]) {
        Some("the API key was rejected; check the key env var for this provider (`oxgraft models` lists them)")
    } else if has(&["http 404", "model not found", "not_found_error", "no such model", "does not exist"]) {
        Some(
            "the model name wasn't found; check it with the provider (`oxgraft models` shows the format; `ollama list` for Ollama)",
        )
    } else if has(&[
        "connection refused",
        "network error calling http://localhost",
        "network error calling http://127.0.0.1",
    ]) {
        Some("nothing answered on the local server; is it running (`ollama serve`, LM Studio's server)?")
    } else if has(&["context length", "context window", "maximum context", "prompt is too long", "too many tokens"]) {
        Some(
            "the conversation is larger than the model's window; set `context_window` lower so Oxgraft compacts earlier",
        )
    } else if has(&["http 429", "rate limit", "overloaded"]) {
        Some("the provider is rate-limiting or overloaded; wait, then `oxgraft run --resume last \"continue\"`")
    } else {
        None
    }
}

pub fn models() {
    println!("Model strings are provider/model. Credentials come from environment variables.\n");
    println!("{:<12} {:<34} {:<26} EXAMPLE", "PROVIDER", "BASE URL (override env)", "API KEY ENV");
    println!(
        "{:<12} {:<34} {:<26} anthropic/{}",
        "anthropic",
        "https://api.anthropic.com",
        "ANTHROPIC_API_KEY",
        oxgraft_providers::anthropic::DEFAULT_MODEL
    );
    for p in oxgraft_providers::openai::PRESETS {
        println!("{:<12} {:<34} {:<26} {}", p.id, p.base_env, p.key_env.unwrap_or("(none, local)"), p.example);
    }
    println!("{:<12} {:<34} {:<26} mock:tests/fixtures/hello.json", "mock", "-", "-");
    println!("\nShort names:");
    for (alias, full) in oxgraft_providers::ALIASES {
        println!("  {alias:<8} {full}");
    }
    println!("\nSet a default in .oxgraft/config.toml or ~/.config/oxgraft/config.toml:  model = \"provider/model\"");
}

pub fn strategies(json: bool) -> Result<(), String> {
    let catalog = oxgraft_strategies::catalog();
    if json {
        println!("{}", serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?);
        return Ok(());
    }
    println!("{:<12} {:<22} {:<11} {:<10} SUMMARY", "SEAM", "ID", "ORIGIN", "DONOR");
    for s in catalog {
        let origin = serde_json::to_value(s.provenance.origin).map_err(|e| e.to_string())?;
        println!(
            "{:<12} {:<22} {:<11} {:<10} {}",
            s.seam.as_str(),
            s.id,
            origin.as_str().unwrap_or("?"),
            s.provenance.harness.unwrap_or("-"),
            s.summary
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::failure_hint;

    #[test]
    fn terminal_controls_are_stripped() {
        use super::plain;
        assert_eq!(plain("a\u{1b}[2Jb\u{1b}[31mc\u{1b}[0m"), "abc");
        assert_eq!(plain("x\u{1b}]0;title\u{7}y"), "xy");
        assert_eq!(plain("x\u{1b}]8;;http://e\u{1b}\\link\u{1b}]8;;\u{1b}\\y"), "xlinky");
        assert_eq!(plain("over\rwrite\u{8}\u{7}"), "overwrite");
        assert_eq!(plain("keep\n\tlines"), "keep\n\tlines");
    }

    #[test]
    fn hints_for_common_provider_failures() {
        let cases = [
            ("ollama: HTTP 400: registry.ollama.ai/library/gemma does not support tools", "can't call tools"),
            ("anthropic: HTTP 401: invalid x-api-key", "API key"),
            ("openai: HTTP 404: The model `gpt-x` does not exist", "model name"),
            (
                "ollama: network error calling http://localhost:11434/v1/chat/completions: Connection refused",
                "local server",
            ),
            ("anthropic: HTTP 400: prompt is too long: 210000 tokens", "context_window"),
            ("anthropic: HTTP 529: Overloaded", "rate-limiting"),
        ];
        for (reason, expect) in cases {
            assert!(failure_hint(reason).is_some_and(|h| h.contains(expect)), "{reason}");
        }
        assert!(failure_hint("reached max turns (50)").is_none());
    }
}
