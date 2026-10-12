use std::collections::HashMap;

use common::http_sig::create_request_headers;
use tracing::debug;

use crate::request::Request;

pub(crate) fn get_http_sig_headers(request: &Request) -> HashMap<String, String> {
    let sig_headers = create_request_headers(&request.url);

    debug!(
        "HTTP signatures\n{}\n{}\n{:?}\n{}",
        sig_headers.signature,
        sig_headers.signature_input,
        sig_headers.signature_agent,
        request.url
    );

    let mut headers = HashMap::from_iter([
        ("Signature".to_string(), sig_headers.signature),
        ("Signature-Input".to_string(), sig_headers.signature_input),
    ]);

    if let Some(sig_agent) = sig_headers.signature_agent {
        headers.insert("Signature-Agent".to_string(), sig_agent);
    }

    headers
}
