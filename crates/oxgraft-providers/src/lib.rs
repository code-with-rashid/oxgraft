//! Model providers (SPEC-0001). Pick one with a model string:
//! `anthropic/claude-opus-5-5`, `openai/<model>`, `ollama/<model>`,
//! `openrouter/<vendor>/<model>`, `mock:<script.json>`, ...

pub mod anthropic;
mod http;
pub mod mock;
pub mod openai;

use oxgraft_seams::{ErrorKind, Provider, ProviderError};

/// A parsed model string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub model: String,
}

/// Parse `provider/model` (the model part may itself contain `/`) or `mock:<path>`.
pub fn parse_model(spec: &str) -> Result<ModelRef, ProviderError> {
    let spec = spec.trim();
    if let Some(path) = spec.strip_prefix("mock:") {
        return Ok(ModelRef { provider: mock::ID.into(), model: path.into() });
    }
    let (provider, model) = spec.split_once('/').ok_or_else(|| {
        ProviderError::new(
            "oxgraft",
            ErrorKind::Config,
            format!("model `{spec}` must look like provider/model, e.g. anthropic/{}", anthropic::DEFAULT_MODEL),
        )
    })?;
    if model.is_empty() {
        return Err(ProviderError::new("oxgraft", ErrorKind::Config, format!("model `{spec}` has no model name")));
    }
    if provider != anthropic::ID && !openai::PRESETS.iter().any(|p| p.id == provider) {
        return Err(ProviderError::new(
            "oxgraft",
            ErrorKind::Config,
            format!("unknown provider `{provider}` (known: {})", known_providers().join(", ")),
        ));
    }
    Ok(ModelRef { provider: provider.into(), model: model.into() })
}

pub fn known_providers() -> Vec<&'static str> {
    let mut ids = vec![anthropic::ID];
    ids.extend(openai::PRESETS.iter().map(|p| p.id));
    ids.push("mock:<script.json>");
    ids
}

/// Options that some providers honour.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Anthropic `output_config.effort` (low, medium, high, xhigh, max).
    pub effort: Option<String>,
}

/// Build the provider for a model reference.
pub fn build(model: &ModelRef, options: &Options) -> Result<Box<dyn Provider>, ProviderError> {
    if model.provider == mock::ID {
        return Ok(Box::new(mock::Mock::from_file(&model.model)?));
    }
    if model.provider == anthropic::ID {
        return Ok(Box::new(anthropic::Anthropic::from_env(options.effort.clone())?));
    }
    let preset = openai::PRESETS.iter().find(|p| p.id == model.provider).copied().ok_or_else(|| {
        ProviderError::new("oxgraft", ErrorKind::Config, format!("unknown provider {}", model.provider))
    })?;
    Ok(Box::new(openai::OpenAiCompatible::from_env(preset)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPEC-0001 AC1
    #[test]
    fn parses_model_strings() {
        assert_eq!(
            parse_model("anthropic/claude-opus-5-5").unwrap(),
            ModelRef { provider: "anthropic".into(), model: "claude-opus-5-5".into() }
        );
        assert_eq!(parse_model("openrouter/vendor/model").unwrap().model, "vendor/model");
        assert_eq!(parse_model("mock:tests/a.json").unwrap().provider, "mock");
        let err = parse_model("nope/x").unwrap_err();
        assert!(err.message.contains("anthropic"), "{err}");
        assert!(parse_model("noslash").is_err());
        assert!(parse_model("openai/").is_err());
    }
}
