use std::{env::var, sync::OnceLock};

use common::constants::{EMAIL_HOST, EMAIL_SENDER_PASS, EMAIL_SENDER_USER};
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials};

static EMAIL_CLIENT: OnceLock<AsyncSmtpTransport<Tokio1Executor>> = OnceLock::new();

pub(crate) async fn get_email_client() -> &'static AsyncSmtpTransport<Tokio1Executor> {
    EMAIL_CLIENT.get_or_init(|| {
        let host = var(EMAIL_HOST).expect("{EMAIL_HOST} to be defined");
        let sender_user = var(EMAIL_SENDER_USER).expect("{EMAIL_SENDER_USER} to be defined");
        let sender_pass = var(EMAIL_SENDER_PASS).expect("{EMAIL_SENDER_PASS} to be defined");

        // TODO: handle unwrap
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .unwrap()
            .credentials(Credentials::new(sender_user, sender_pass))
            .build()
    })
}
