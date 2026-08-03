use std::time::Duration;

use reqwest::Response;
use reqwest_middleware::Error;
use reqwest_retry::{
    RetryTransientMiddleware, Retryable, RetryableStrategy, default_on_request_failure,
    policies::ExponentialBackoff,
};
use tracing::warn;

const PAGE_MIN_SECS_BACKOFF: u64 = 10;
const PAGE_MAX_SECS_BACKOFF: u64 = 60;
const MAX_RETRY: u32 = 10;

pub(crate) struct RetryStrategy;

impl RetryableStrategy for RetryStrategy {
    fn handle(&self, response: &Result<Response, Error>) -> Option<Retryable> {
        match response {
            Ok(finished_request) => {
                if finished_request.status().is_client_error()
                    || finished_request.status().is_server_error()
                {
                    warn!(
                        "Retrying request for {}, had gotten {}",
                        finished_request.url(),
                        finished_request.status()
                    );

                    return Some(Retryable::Transient);
                }

                None
            }
            Err(err) => default_on_request_failure(err),
        }
    }
}

pub(crate) fn get_retry_middleware() -> RetryTransientMiddleware<ExponentialBackoff, RetryStrategy>
{
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(
            Duration::from_secs(PAGE_MIN_SECS_BACKOFF),
            Duration::from_secs(PAGE_MAX_SECS_BACKOFF),
        )
        .build_with_max_retries(MAX_RETRY);

    RetryTransientMiddleware::new_with_policy_and_strategy(retry_policy, RetryStrategy)
}
