use super::types::NetConfig;
use crate::prelude::*;
use url_serde::SerdeUrl;

#[derive(Debug, Deserialize)]
pub struct WebsocketStartResponse {
    pub url: SerdeUrl,
    pub expires: f32,
}

pub async fn start_ws(cfg: &NetConfig) -> Fallible<WebsocketStartResponse> {
    surf::post(&cfg.node)
        .recv_json::<WebsocketStartResponse>()
        .await
        .map_err(|e| failure::Error::from_boxed_compat(e))
}
