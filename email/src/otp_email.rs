use std::env::var;

use common::{
    constants::{EMAIL_REPLY_USER, EMAIL_SENDER_USER},
    utils::get_frontend_domain,
};
use lettre::{AsyncTransport, Message, message::header::ContentType};

use crate::client::get_email_client;

const EMAIL: &str = include_str!("./templates/otp.html");

// TODO: handle unwraps in this method, this should be fine for now
// since backend API will return 5xx error

/// Returns true if email sent successfully
pub async fn send_otp_email(email: &str, code: &str) -> bool {
    let reply_user = var(EMAIL_REPLY_USER).expect("{EMAIL_REPLY_USER} to be defined");
    let sender_user = var(EMAIL_SENDER_USER).expect("{EMAIL_SENDER_USER} to be defined");

    let title = format!("AlmostProhibited.ca - {code} is your one-time code");

    let email = Message::builder()
        .from(
            format!("<{sender_user}>")
                .parse()
                .expect("Valid sender mailbox"),
        )
        .reply_to(
            format!("<{reply_user}>")
                .parse()
                .expect("Valid reply mailbox"),
        )
        .to(format!("<{email}>").parse().expect("Valid mailbox"))
        .subject(&title)
        .header(ContentType::TEXT_HTML)
        .body(
            EMAIL
                .replace("{{domain}}", &get_frontend_domain())
                .replace("{{otp-code}}", code)
                .replace("{{title}}", &title),
        )
        .unwrap();

    let response = get_email_client().await.send(email).await.unwrap();

    response.is_positive()
}
