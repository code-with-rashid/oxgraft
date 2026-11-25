//! Provider retries with exponential backoff (SPEC-0001 AC5).

use std::time::Duration;

use oxgraft_seams::{Completion, CompletionRequest, Provider, ProviderError};

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base: Duration,
    pub max_wait: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 6, base: Duration::from_millis(1_000), max_wait: Duration::from_secs(60) }
    }
}

impl RetryPolicy {
    /// Wait before retry number `attempt` (1-based), honouring a server hint.
    pub fn wait(&self, attempt: u32, hint_ms: Option<u64>) -> Duration {
        if let Some(ms) = hint_ms {
            return Duration::from_millis(ms).min(self.max_wait);
        }
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1).min(16));
        let jitter = Duration::from_millis(u64::from(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_millis()).unwrap_or(0)
                % 250,
        ));
        (self.base * factor + jitter).min(self.max_wait)
    }
}

/// Call the provider, retrying transient failures. `on_retry(attempt, wait, error)` is called before each wait.
pub fn complete_with_retries(
    provider: &dyn Provider,
    request: &CompletionRequest,
    policy: &RetryPolicy,
    sleep: &dyn Fn(Duration),
    on_retry: &mut dyn FnMut(u32, Duration, &ProviderError),
) -> Result<Completion, ProviderError> {
    with_retries(policy, sleep, on_retry, &mut || provider.complete(request))
}

/// Stream from the provider (ADR 0007), retrying transient failures. Text already
/// delivered by a failed attempt is followed by `on_retry`, so renderers can start over.
pub fn stream_with_retries(
    provider: &dyn Provider,
    request: &CompletionRequest,
    policy: &RetryPolicy,
    sleep: &dyn Fn(Duration),
    on_retry: &mut dyn FnMut(u32, Duration, &ProviderError),
    on_text: &mut dyn FnMut(&str),
) -> Result<Completion, ProviderError> {
    with_retries(policy, sleep, on_retry, &mut || provider.stream(request, on_text))
}

fn with_retries(
    policy: &RetryPolicy,
    sleep: &dyn Fn(Duration),
    on_retry: &mut dyn FnMut(u32, Duration, &ProviderError),
    call: &mut dyn FnMut() -> Result<Completion, ProviderError>,
) -> Result<Completion, ProviderError> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        match call() {
            Ok(c) => return Ok(c),
            Err(e) if e.retryable() && attempt < policy.max_attempts => {
                let wait = policy.wait(attempt, e.retry_after_ms);
                on_retry(attempt, wait, &e);
                sleep(wait);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxgraft_seams::{ErrorKind, Message, StopReason, Usage};
    use std::sync::Mutex;

    struct Flaky {
        failures: Mutex<Vec<ErrorKind>>,
    }

    impl Provider for Flaky {
        fn id(&self) -> &str {
            "flaky"
        }
        fn complete(&self, _r: &CompletionRequest) -> Result<Completion, ProviderError> {
            match self.failures.lock().unwrap().pop() {
                Some(kind) => Err(ProviderError::new("flaky", kind, "nope")),
                None => Ok(Completion { content: vec![], stop: StopReason::EndTurn, usage: Usage::default() }),
            }
        }
    }

    fn req() -> CompletionRequest {
        CompletionRequest {
            model: "m".into(),
            system: String::new(),
            messages: vec![Message::user("x")],
            tools: vec![],
            max_tokens: 1,
        }
    }

    // SPEC-0001 AC5
    #[test]
    fn retries_transient_then_succeeds() {
        let p = Flaky { failures: Mutex::new(vec![ErrorKind::Transient, ErrorKind::Transient]) };
        let mut retries = 0;
        let r = complete_with_retries(&p, &req(), &RetryPolicy::default(), &|_| {}, &mut |_, _, _| retries += 1);
        assert!(r.is_ok());
        assert_eq!(retries, 2);
    }

    #[test]
    fn does_not_retry_auth_errors_and_gives_up_eventually() {
        let p = Flaky { failures: Mutex::new(vec![ErrorKind::Auth]) };
        let mut retries = 0;
        assert!(
            complete_with_retries(&p, &req(), &RetryPolicy::default(), &|_| {}, &mut |_, _, _| retries += 1).is_err()
        );
        assert_eq!(retries, 0);
        let p = Flaky { failures: Mutex::new(vec![ErrorKind::Transient; 10]) };
        let policy = RetryPolicy { max_attempts: 3, ..RetryPolicy::default() };
        let mut retries = 0;
        assert!(complete_with_retries(&p, &req(), &policy, &|_| {}, &mut |_, _, _| retries += 1).is_err());
        assert_eq!(retries, 2);
    }

    #[test]
    fn waits_grow_and_respect_hints() {
        let p = RetryPolicy::default();
        assert!(p.wait(3, None) >= Duration::from_secs(4));
        assert_eq!(p.wait(1, Some(1500)), Duration::from_millis(1500));
        assert_eq!(p.wait(30, None), p.max_wait);
        assert_eq!(p.wait(1, Some(10_000_000)), p.max_wait);
    }
}
