//! `inert_reads`: ordered rules, plus auto-allow for shell commands that can only
//! read (GRAFT-0047). Clean-room from Crush's documented behaviour (FSL-1.1-MIT:
//! no source consulted): a command runs without asking only when the whole line is
//! one simple, known read-only command with no redirection, substitution,
//! pipeline, chaining, assignment or mutating argument.

use oxgraft_seams::{Action, ApprovalPolicy, Decision, Origin, Provenance, Seam, Strategy, StrategyInfo};

use super::OrderedRules;

/// Characters that make a command line more than one inert simple command.
const META: &[char] = &['>', '<', '|', ';', '&', '$', '`', '(', ')', '{', '}', '\n', '\r', '\\', '*', '?'];

/// Read-only commands whose arguments can't make them write, run code or reach the network.
const READERS: &[&str] = &[
    "ls", "pwd", "cat", "head", "tail", "wc", "grep", "rg", "tree", "file", "stat", "which", "du", "basename",
    "dirname", "realpath", "date", "whoami", "uname", "echo", "diff", "cut", "sort", "uniq", "nl",
];

/// `git` subcommands that only read.
const GIT_READERS: &[&str] =
    &["status", "diff", "log", "show", "blame", "ls-files", "rev-parse", "shortlog", "describe"];

/// Whether `command` is a single inert read-only command (see the module docs).
pub fn is_inert(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() || command.contains(META) {
        return false;
    }
    let words: Vec<&str> = command.split_whitespace().collect();
    let (name, args) = (words[0], &words[1..]);
    // Reads stay inside the workspace, like the read tool: no absolute, home or parent paths.
    let outside = |a: &&str| {
        let a = a.trim_matches(['"', '\'']);
        let path = a.split_once('=').map_or(a, |(_, v)| v);
        path.starts_with('/') || path.starts_with('~') || path.contains("..") || path.get(1..2) == Some(":")
    };
    if args.iter().any(outside) {
        return false;
    }
    // Assignments (`FOO=1 ls`) and option-looking or quoted command names.
    if name.contains('=') || name.starts_with('-') || name.contains(['"', '\'']) {
        return false;
    }
    let mutating = |a: &&str| {
        a.starts_with("--output")
            || a.starts_with("--exec")
            || a.starts_with("--pre")
            || matches!(*a, "-o" | "--in-place" | "-i" | "--delete" | "-delete" | "--ext-diff" | "--textconv")
    };
    match name {
        "git" => {
            // `git -c key=value …` and `git --git-dir …` change what runs.
            let Some(sub) = args.first() else { return false };
            GIT_READERS.contains(sub) && !args.iter().any(mutating)
        }
        "find" => !args.iter().any(|a| {
            matches!(
                *a,
                "-exec" | "-execdir" | "-ok" | "-okdir" | "-delete" | "-fprint" | "-fprint0" | "-fprintf" | "-fls"
            )
        }),
        // `sort -o file` writes; `rg --pre cmd` runs code; `sort --compress-program` runs code.
        "sort" => !args.iter().any(|a| mutating(a) || a.starts_with("--compress")),
        _ => READERS.contains(&name) && !args.iter().any(mutating),
    }
}

/// Ordered rules, with inert read-only shell commands allowed instead of asked.
#[derive(Debug, Clone)]
pub struct InertReads {
    rules: OrderedRules,
}

impl InertReads {
    pub fn new(rules: OrderedRules) -> Self {
        Self { rules }
    }
}

impl Default for InertReads {
    fn default() -> Self {
        Self::new(OrderedRules::default())
    }
}

static INERT_READS: StrategyInfo = StrategyInfo {
    id: "inert_reads",
    seam: Seam::Approval,
    summary: "ordered_rules, plus auto-allow for single read-only shell commands (ls, cat, git status, grep…) with nothing that could write or run code.",
    provenance: Provenance {
        origin: Origin::CleanRoom,
        harness: Some("crush"),
        license: Some("FSL-1.1-MIT"),
        graft: Some("GRAFT-0047"),
        notes: "Behaviour from Crush's README only; FSL license, so no source was consulted.",
    },
};

impl Strategy for InertReads {
    fn info(&self) -> &StrategyInfo {
        &INERT_READS
    }
}

impl ApprovalPolicy for InertReads {
    fn decide(&self, action: &Action) -> Decision {
        let decision = self.rules.decide(action);
        if action.tool == "bash" && decision == Decision::Ask && is_inert(&action.resource) {
            // Arguments the rules would deny reading (e.g. `cat .env`) still ask.
            let words: Vec<&str> = action.resource.split_whitespace().skip(1).collect();
            let denied = words.iter().any(|w| {
                let path = w.trim_matches(['"', '\'']);
                self.rules.decide(&Action { tool: "read".into(), resource: path.into() }) == Decision::Deny
            });
            if !denied {
                return Decision::Allow;
            }
        }
        decision
    }

    fn explain(&self, action: &Action) -> Option<String> {
        self.rules.explain(action)
    }

    fn needs_fresh_yes(&self, action: &Action) -> Option<String> {
        self.rules.needs_fresh_yes(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::Rule;

    fn bash(c: &str) -> Action {
        Action { tool: "bash".into(), resource: c.into() }
    }

    #[test]
    fn recognises_inert_commands() {
        for c in [
            "ls -la src",
            "git status",
            "git diff HEAD~1",
            "git log --oneline -5",
            "grep -rn TODO src",
            "cat README.md",
            "find . -name Cargo.toml",
            "wc -l src/main.rs",
            "rg fn crates",
        ] {
            assert!(is_inert(c), "{c} should be inert");
        }
        for c in [
            "ls > out.txt",
            "cat a | sh",
            "ls; rm x",
            "git status && git push",
            "echo $(id)",
            "echo `id`",
            "FOO=1 ls",
            "git commit -m x",
            "git -c core.pager=evil log",
            "git diff --output=x",
            "git diff --ext-diff",
            "find . -delete",
            "find . -exec rm {} ;",
            "sort -o out.txt in.txt",
            "rg --pre ./evil x",
            "cargo test",
            "rm -rf x",
            "python3 -c 1",
            "cat ~/.ssh/id_rsa",
            "ls *",
            "cat /etc/passwd",
            "cat ../secret",
            "head C:/Users/x",
            "",
        ] {
            assert!(!is_inert(c), "{c} should not be inert");
        }
    }

    #[test]
    fn inert_commands_are_allowed_but_rules_still_win() {
        // SPEC-0002 AC8
        let p = InertReads::new(OrderedRules::with_extra(vec![Rule::new("bash", "git log*", Decision::Deny)]));
        assert_eq!(p.decide(&bash("git status")), Decision::Allow);
        assert_eq!(p.decide(&bash("cargo test")), Decision::Ask);
        assert_eq!(p.decide(&bash("git log")), Decision::Deny, "a deny rule beats inert auto-allow");
        assert_ne!(p.decide(&bash("cat .env")), Decision::Allow, "paths the rules deny reading are not auto-allowed");
        assert_eq!(p.decide(&bash("cat .env README.md")), Decision::Ask);
        assert_eq!(p.decide(&Action { tool: "edit".into(), resource: "a.rs".into() }), Decision::Ask);
    }
}
