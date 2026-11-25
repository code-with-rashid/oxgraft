//! The Oxgraft core. Knows seams, not strategies: the binary injects every
//! available strategy and a [`Profile`] picks one per seam.

pub mod agent;
pub mod checkpoint;
pub mod hooks;
pub mod prompt;
pub mod retry;
pub mod schema;
pub mod session;

pub use agent::{Agent, AgentConfig, Answer, Approver, Fixed, Outcome, Status};
pub use session::Session;

use oxgraft_protocol::{Event, PROTOCOL_VERSION};
use oxgraft_seams::{ApprovalPolicy, ContextStrategy, EditFormat, Sandbox, Seam, Strategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("invalid profile: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("profile selects unknown {seam} strategy `{id}` (available: {available})")]
    UnknownStrategy { seam: Seam, id: String, available: String },
}

/// Which strategy to use for each seam. Stored in `profiles/*.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub edit_format: String,
    pub context: String,
    pub approval: String,
    pub sandbox: String,
}

impl Selection {
    pub fn get(&self, seam: Seam) -> &str {
        match seam {
            Seam::EditFormat => &self.edit_format,
            Seam::Context => &self.context,
            Seam::Approval => &self.approval,
            Seam::Sandbox => &self.sandbox,
        }
    }
}

/// A named combination of strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub strategies: Selection,
}

impl Profile {
    pub fn from_toml(text: &str) -> Result<Self, ProfileError> {
        Ok(toml::from_str(text)?)
    }
}

/// Every strategy the binary makes available, per seam.
pub struct Available {
    pub edit_formats: Vec<Box<dyn EditFormat>>,
    pub contexts: Vec<Box<dyn ContextStrategy>>,
    pub approvals: Vec<Box<dyn ApprovalPolicy>>,
    pub sandboxes: Vec<Box<dyn Sandbox>>,
}

/// The strategies a session runs with: exactly one per seam.
pub struct Toolkit {
    pub profile: Profile,
    pub edit_format: Box<dyn EditFormat>,
    pub context: Box<dyn ContextStrategy>,
    pub approval: Box<dyn ApprovalPolicy>,
    pub sandbox: Box<dyn Sandbox>,
}

impl std::fmt::Debug for Toolkit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toolkit")
            .field("profile", &self.profile.name)
            .field("edit_format", &self.edit_format.info().id)
            .field("context", &self.context.info().id)
            .field("approval", &self.approval.info().id)
            .field("sandbox", &self.sandbox.info().id)
            .finish()
    }
}

impl std::fmt::Debug for Available {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Available").finish_non_exhaustive()
    }
}

fn pick<T: ?Sized + Strategy>(items: Vec<Box<T>>, seam: Seam, id: &str) -> Result<Box<T>, ProfileError> {
    let available = items.iter().map(|s| s.info().id).collect::<Vec<_>>().join(", ");
    items.into_iter().find(|s| s.info().id == id).ok_or_else(|| ProfileError::UnknownStrategy {
        seam,
        id: id.to_owned(),
        available,
    })
}

impl Toolkit {
    pub fn assemble(profile: Profile, available: Available) -> Result<Self, ProfileError> {
        let s = &profile.strategies;
        let edit_format = pick(available.edit_formats, Seam::EditFormat, s.get(Seam::EditFormat))?;
        let context = pick(available.contexts, Seam::Context, s.get(Seam::Context))?;
        let approval = pick(available.approvals, Seam::Approval, s.get(Seam::Approval))?;
        let sandbox = pick(available.sandboxes, Seam::Sandbox, s.get(Seam::Sandbox))?;
        Ok(Self { profile, edit_format, context, approval, sandbox })
    }

    /// The opening events of a session: which profile and strategies are in force.
    /// Arena runs record these so every result is tied to an exact configuration.
    pub fn session_preamble(&self) -> Vec<Event> {
        let mut events = vec![Event::SessionStarted {
            protocol: PROTOCOL_VERSION,
            profile: self.profile.name.clone(),
            session_id: None,
            model: None,
        }];
        let ids = [
            (Seam::EditFormat, self.edit_format.info().id),
            (Seam::Context, self.context.info().id),
            (Seam::Approval, self.approval.info().id),
            (Seam::Sandbox, self.sandbox.info().id),
        ];
        events.extend(
            ids.into_iter()
                .map(|(seam, id)| Event::StrategySelected { seam: seam.as_str().into(), strategy: id.into() }),
        );
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::*;

    #[derive(Debug)]
    struct Fake(StrategyInfo);
    impl Strategy for Fake {
        fn info(&self) -> &StrategyInfo {
            &self.0
        }
    }
    impl EditFormat for Fake {
        fn apply(&self, _o: &str, e: &str) -> Result<String, EditError> {
            Ok(e.into())
        }
        fn instructions(&self) -> &'static str {
            ""
        }
    }
    impl ContextStrategy for Fake {
        fn select(&self, _i: &[ContextItem], _b: usize) -> Vec<String> {
            vec![]
        }
    }
    impl ApprovalPolicy for Fake {
        fn decide(&self, _a: &Action) -> Decision {
            Decision::Ask
        }
    }
    impl Sandbox for Fake {
        fn confinement(&self, _c: &Command) -> Confinement {
            Confinement::None
        }
    }

    fn fake(seam: Seam, id: &'static str) -> Fake {
        Fake(StrategyInfo { id, seam, summary: "", provenance: Provenance::original("test") })
    }

    fn available() -> Available {
        Available {
            edit_formats: vec![Box::new(fake(Seam::EditFormat, "e1")), Box::new(fake(Seam::EditFormat, "e2"))],
            contexts: vec![Box::new(fake(Seam::Context, "c1"))],
            approvals: vec![Box::new(fake(Seam::Approval, "a1"))],
            sandboxes: vec![Box::new(fake(Seam::Sandbox, "s1"))],
        }
    }

    const PROFILE: &str = r#"
name = "test"
[strategies]
edit_format = "e2"
context = "c1"
approval = "a1"
sandbox = "s1"
"#;

    #[test]
    fn assembles_the_selected_strategies() {
        let toolkit = Toolkit::assemble(Profile::from_toml(PROFILE).unwrap(), available()).unwrap();
        assert_eq!(toolkit.edit_format.info().id, "e2");
        let preamble = toolkit.session_preamble();
        assert_eq!(preamble.len(), 1 + Seam::ALL.len());
        assert_eq!(
            preamble[0],
            Event::SessionStarted { protocol: PROTOCOL_VERSION, profile: "test".into(), session_id: None, model: None }
        );
    }

    #[test]
    fn unknown_strategy_lists_alternatives() {
        let text = PROFILE.replace("\"e2\"", "\"nope\"");
        let err = Toolkit::assemble(Profile::from_toml(&text).unwrap(), available()).unwrap_err();
        assert!(err.to_string().contains("e1, e2"), "{err}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text = format!("{PROFILE}typo = 1\n");
        assert!(Profile::from_toml(&text).is_err());
    }
}
