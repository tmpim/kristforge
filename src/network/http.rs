use super::NetworkError;
use isahc::http::Uri;
use isahc::ResponseExt;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct WsStartResponse {
    pub url: Url,
}

/// Request to start a websocket connection
pub async fn ws_start(uri: Uri) -> Result<WsStartResponse, NetworkError> {
    let json = isahc::post_async(uri, ()).await?.text_async().await?;
    serde_json::from_str(&json).map_err(|e| e.into())
}
