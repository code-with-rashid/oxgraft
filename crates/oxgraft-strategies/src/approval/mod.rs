use oxgraft_seams::{Action, ApprovalPolicy, Decision, Origin, Provenance, Seam, Strategy, StrategyInfo};

/// One rule: `tool` and `resource` are `*` wildcard patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub tool: String,
    pub resource: String,
    pub decision: Decision,
}

impl Rule {
    pub fn new(tool: &str, resource: &str, decision: Decision) -> Self {
        Self { tool: tool.into(), resource: resource.into(), decision }
    }
}

/// Ordered allow/ask/deny rules per tool and resource; the last matching rule wins.
#[derive(Debug, Clone)]
pub struct OrderedRules {
    rules: Vec<Rule>,
    fallback: Decision,
}

static ORDERED_RULES: StrategyInfo = StrategyInfo {
    id: "ordered_rules",
    seam: Seam::Approval,
    summary: "Ordered allow/ask/deny rules by tool and resource pattern; last match wins; default ask.",
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
}

impl Default for OrderedRules {
    fn default() -> Self {
        Self::new(
            vec![
                Rule::new("read", "*", Decision::Allow),
                Rule::new("grep", "*", Decision::Allow),
                Rule::new("glob", "*", Decision::Allow),
                Rule::new("edit", "*", Decision::Ask),
                Rule::new("bash", "*", Decision::Ask),
                Rule::new("*", "*.env", Decision::Deny),
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
        self.rules
            .iter()
            .rev()
            .find(|r| wildcard(&r.tool, &action.tool) && wildcard(&r.resource, &action.resource))
            .map_or(self.fallback, |r| r.decision)
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
