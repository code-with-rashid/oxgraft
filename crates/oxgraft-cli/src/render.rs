//! Human-readable output. Assistant text goes to stdout; activity to stderr.

use oxgraft_protocol::Event;

#[derive(Debug, Default)]
pub struct Human {
    tools: std::collections::HashMap<String, String>,
    /// Text deltas have been printed for the reply in progress (ADR 0007).
    streaming: bool,
}

fn one_line(s: &str, max: usize) -> String {
    let line = s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    if line.chars().count() > max {
        format!("{}…", line.chars().take(max).collect::<String>())
    } else {
        line.to_string()
    }
}

impl Human {
    pub fn event(&mut self, event: &Event) {
        use std::io::Write;
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
                print!("{text}");
                let _ = std::io::stdout().flush();
            }
            Event::Message { text, .. } => println!("\n{}", text.trim()),
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
            }
            _ => {}
        }
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
