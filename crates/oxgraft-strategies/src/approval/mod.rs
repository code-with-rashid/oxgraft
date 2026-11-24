use oxgraft_seams::{Action, ApprovalPolicy, Decision, Origin, Provenance, Seam, Strategy, StrategyInfo};

/// One rule: `tool` and `resource` are `*` wildcard patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub tool: String,
    pub resource: String,
    pub decision: Decision,
    /// Shown to the model when the rule denies, e.g. "use the read tool instead" (Codex execpolicy).
    pub justification: Option<String>,
}

impl Rule {
    pub fn new(tool: &str, resource: &str, decision: Decision) -> Self {
        Self { tool: tool.into(), resource: resource.into(), decision, justification: None }
    }

    pub fn because(mut self, why: &str) -> Self {
        self.justification = Some(why.into());
        self
    }

    fn matches(&self, tool: &str, resource: &str) -> bool {
        wildcard(&self.tool, tool) && wildcard(&self.resource, resource)
    }
}

/// Allow/ask/deny rules per tool and resource pattern.
///
/// - Any matching `deny` wins, whatever its position or specificity (Claude Code's
///   ordering; a narrow allow can never punch a hole in a deny).
/// - Otherwise the last matching rule wins (OpenCode's ordering), else `fallback`.
/// - Shell commands are split on `&&`, `||`, `;`, `|` and newlines; every part must
///   pass and the most restrictive decision applies, so `cargo *` cannot approve
///   `cargo test && rm -rf ~`. Command substitution (`$(`, backticks) is at least `ask`.
#[derive(Debug, Clone)]
pub struct OrderedRules {
    rules: Vec<Rule>,
    fallback: Decision,
}

static ORDERED_RULES: StrategyInfo = StrategyInfo {
    id: "ordered_rules",
    seam: Seam::Approval,
    summary: "allow/ask/deny rules by tool and resource; deny always wins, else last match; shell commands checked per part.",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("opencode"),
        license: Some("MIT"),
        graft: Some("GRAFT-0002"),
        notes: "Idea from OpenCode's per-action, per-resource permission rules; re-implemented from its docs.",
    },
};

impl OrderedRules {
    pub fn new(rules: Vec<Rule>, fallback: Decision) -> Self {
        Self { rules, fallback }
    }

    /// The default rules followed by `extra` (e.g. from user config).
    pub fn with_extra(extra: Vec<Rule>) -> Self {
        let mut rules = Self::default().rules;
        rules.extend(extra);
        Self::new(rules, Decision::Ask)
    }

    fn decide_one(&self, tool: &str, resource: &str) -> (Decision, Option<&Rule>) {
        if let Some(deny) = self.rules.iter().find(|r| r.decision == Decision::Deny && r.matches(tool, resource)) {
            return (Decision::Deny, Some(deny));
        }
        match self.rules.iter().rev().find(|r| r.matches(tool, resource)) {
            Some(r) => (r.decision, Some(r)),
            None => (self.fallback, None),
        }
    }

    fn decide_with_rule(&self, action: &Action) -> (Decision, Option<&Rule>) {
        if action.tool != "bash" {
            return self.decide_one(&action.tool, &action.resource);
        }
        let parts = split_command(&action.resource);
        let mut worst = (Decision::Allow, None);
        for part in if parts.is_empty() { vec![action.resource.trim().to_string()] } else { parts } {
            let d = self.decide_one("bash", &part);
            if severity(d.0) > severity(worst.0) {
                worst = d;
            }
        }
        // Whole-command rules (e.g. deny `*rm -rf*`) apply too.
        let whole = self.decide_one("bash", action.resource.trim());
        if whole.0 == Decision::Deny {
            return whole;
        }
        if worst.0 == Decision::Allow && (action.resource.contains("$(") || action.resource.contains('`')) {
            return (Decision::Ask, None);
        }
        worst
    }
}

fn severity(d: Decision) -> u8 {
    match d {
        Decision::Allow => 0,
        Decision::Ask => 1,
        Decision::Deny => 2,
    }
}

/// Split a shell command line into its simple commands (conservatively: quotes are
/// not honoured, which can only produce more parts, never fewer).
pub fn split_command(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        let sep = match c {
            ';' | '\n' => true,
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                true
            }
            '|' => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                true
            }
            _ => false,
        };
        if sep {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_string());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

impl Default for OrderedRules {
    fn default() -> Self {
        Self::new(
            vec![
                Rule::new("read", "*", Decision::Allow),
                Rule::new("ls", "*", Decision::Allow),
                Rule::new("grep", "*", Decision::Allow),
                Rule::new("glob", "*", Decision::Allow),
                Rule::new("todo", "*", Decision::Allow),
                Rule::new("skill", "*", Decision::Allow),
                Rule::new("edit", "*", Decision::Ask),
                Rule::new("write", "*", Decision::Ask),
                Rule::new("bash", "*", Decision::Ask),
                Rule::new("*", "*.env", Decision::Deny)
                    .because("files named .env usually hold secrets; ask the user for the values you need"),
            ],
            Decision::Ask,
        )
    }
}

impl Strategy for OrderedRules {
    fn info(&self) -> &StrategyInfo {
        &ORDERED_RULES
    }
}

impl ApprovalPolicy for OrderedRules {
    fn decide(&self, action: &Action) -> Decision {
        self.decide_with_rule(action).0
    }

    fn explain(&self, action: &Action) -> Option<String> {
        self.decide_with_rule(action).1.and_then(|r| r.justification.clone())
    }
}

/// Approve everything. For sandboxed CI and benchmarks only.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAll;

static ALLOW_ALL: StrategyInfo = StrategyInfo {
    id: "allow_all",
    seam: Seam::Approval,
    summary: "Approve every action. Only safe inside an outer sandbox (CI, arena runs).",
    provenance: Provenance::original("Equivalent of the 'yolo' or full-auto mode most harnesses offer."),
};

impl Strategy for AllowAll {
    fn info(&self) -> &StrategyInfo {
        &ALLOW_ALL
    }
}

impl ApprovalPolicy for AllowAll {
    fn decide(&self, _action: &Action) -> Decision {
        Decision::Allow
    }
}

/// `*` matches any run of characters; everything else matches literally.
fn wildcard(pattern: &str, text: &str) -> bool {
    let (p, t) = (pattern.as_bytes(), text.as_bytes());
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(tool: &str, resource: &str) -> Action {
        Action { tool: tool.into(), resource: resource.into() }
    }

    #[test]
    fn wildcard_matching() {
        assert!(wildcard("*", ""));
        assert!(wildcard("*.env", "config/.env"));
        assert!(wildcard("git *", "git status"));
        assert!(wildcard("a*b*c", "aXXbYc"));
        assert!(!wildcard("*.env", "env.rs"));
        assert!(!wildcard("git *", "rm -rf"));
    }

    #[test]
    fn default_rules() {
        let p = OrderedRules::default();
        assert_eq!(p.decide(&act("read", "src/main.rs")), Decision::Allow);
        assert_eq!(p.decide(&act("bash", "cargo test")), Decision::Ask);
        assert_eq!(p.decide(&act("read", "secrets/.env")), Decision::Deny);
        assert_eq!(p.decide(&act("webfetch", "https://example.com")), Decision::Ask);
    }

    #[test]
    fn deny_always_wins_and_compound_commands_are_checked_per_part() {
        let p = OrderedRules::with_extra(vec![
            Rule::new("bash", "cargo *", Decision::Allow),
            Rule::new("bash", "rm *", Decision::Deny).because("use git clean instead"),
            Rule::new("read", "secrets/*", Decision::Allow),
        ]);
        assert_eq!(p.decide(&act("bash", "cargo test")), Decision::Allow);
        assert_eq!(p.decide(&act("bash", "cargo test && rm -rf ~")), Decision::Deny);
        assert_eq!(p.explain(&act("bash", "cargo test; rm x")).as_deref(), Some("use git clean instead"));
        assert_eq!(p.decide(&act("bash", "cargo test | tee out.txt")), Decision::Ask, "tee is not allowed");
        assert_eq!(p.decide(&act("bash", "cargo $(curl evil)")), Decision::Ask);
        // A later, narrower allow cannot override a deny.
        assert_eq!(p.decide(&act("read", "secrets/.env")), Decision::Deny);
        assert_eq!(split_command("a && b || c; d | e\nf"), vec!["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn last_matching_rule_wins() {
        let p = OrderedRules::new(
            vec![Rule::new("bash", "*", Decision::Ask), Rule::new("bash", "cargo *", Decision::Allow)],
            Decision::Deny,
        );
        assert_eq!(p.decide(&act("bash", "cargo test")), Decision::Allow);
        assert_eq!(p.decide(&act("bash", "rm -rf /")), Decision::Ask);
        assert_eq!(p.decide(&act("edit", "a.rs")), Decision::Deny);
    }
}
