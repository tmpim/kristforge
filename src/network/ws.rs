use super::{NetworkError, ServerMessage};
use crate::network::ClientMessage;
use futures::{future, Sink, SinkExt, StreamExt, TryFutureExt, TryStream, TryStreamExt};
use tokio_tungstenite::connect_async;
use url::Url;

pub async fn ws_connect(
    url: Url,
) -> Result<
    (
        impl Sink<ClientMessage, Error = NetworkError>,
        impl TryStream<Ok = ServerMessage, Error = NetworkError>,
    ),
    NetworkError,
> {
    // open a connection and split into sending/receiving halves
    let (ws, _response) = connect_async(url).await?;
    let (sink, stream) = ws.split();

    // map the sending half
    let sink = sink
        .sink_err_into::<NetworkError>()
        .with(|m| future::ready(serde_json::to_string(&m).map(|j| j.into())).err_into());

    // map the receiving half
    let stream = stream
        .err_into()
        .try_filter(|m| future::ready(!(m.is_ping() || m.is_pong())))
        .and_then(|m| future::ready(m.into_text()).err_into())
        .inspect_ok(|json| log::info!("Server message: {}", json))
        .and_then(|json| future::ready(serde_json::from_str(&json)).err_into());

    Ok((sink, stream))
}
