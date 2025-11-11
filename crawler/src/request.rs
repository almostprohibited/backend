use serde_json::Value;

use crate::traits::HttpMethod;

#[derive(Debug)]
pub struct Request {
    pub(crate) method: HttpMethod,
    pub(crate) url: String,
    pub(crate) json: Option<Value>,
    pub(crate) body: Option<String>,
    pub(crate) headers: Option<Vec<(String, String)>>,
}

pub struct RequestBuilder {
    request: Request,
}

impl Request {
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    pub fn default() -> Self {
        Request {
            method: HttpMethod::GET,
            url: Default::default(),
            json: None,
            body: None,
            headers: None,
        }
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            request: Request::default(),
        }
    }

    pub fn set_method(mut self, method: HttpMethod) -> Self {
        self.request.method = method;

        self
    }

    pub fn set_url(mut self, url: impl Into<String>) -> Self {
        self.request.url = url.into();

        self
    }

    pub fn set_json_body(mut self, json: Value) -> Self {
        self.request.json = Some(json);

        self
    }

    pub fn set_body(mut self, body: String) -> Self {
        self.request.body = Some(body);

        self
    }

    pub fn set_headers(mut self, headers: &[(String, String)]) -> Self {
        self.request.headers = Some(headers.to_vec());

        self
    }

    pub fn build(self) -> Request {
        self.request
    }
}
