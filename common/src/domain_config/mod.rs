pub mod default;
pub mod images;

#[derive(Debug, Clone)]
pub struct ClientSettings {
    pub proxy_type: ProxyType,
    pub client_type: ClientType,
    pub include_http_sigs: bool,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            proxy_type: Default::default(),
            client_type: Default::default(),
            include_http_sigs: true,
        }
    }
}

impl ClientSettings {
    pub fn with_proxy_type(mut self, proxy: ProxyType) -> Self {
        self.proxy_type = proxy;

        self
    }

    pub fn with_client_type(mut self, client: ClientType) -> Self {
        self.client_type = client;

        self
    }

    pub fn without_http_sigs(mut self) -> Self {
        self.include_http_sigs = false;

        self
    }
}

#[derive(Default, Debug, Clone)]
pub enum ProxyType {
    #[default]
    None,
    Normal,
}

#[derive(Default, Debug, Clone)]
pub enum ClientType {
    #[default]
    Normal,
    Emulated,
    Browser,
}
