use super::http::start_ws;
use super::types::NetConfig;
use crate::network::types::{ClientMessage, ServerMessage};
use crate::prelude::*;
use websocket::futures::stream::Stream as _;
use websocket::r#async::client::ClientBuilder;
use websocket::OwnedMessage;

pub async fn connect(
    cfg: &NetConfig,
) -> Fallible<(
    impl Sink<ClientMessage, Error = failure::Error>,
    impl TryStream<Ok = ServerMessage, Error = failure::Error>,
)> {
    // request a websocket connection with the REST API
    let url = start_ws(cfg).await?.url;

    // connect to the given websocket URL
    debug!("Opening websocket connection to {}", &url.as_str());
    let (client, _headers) = ClientBuilder::new(url.as_str())?
        .async_connect(None)
        .compat()
        .await?;

    // split the connection into the respective parts
    let (sink, stream) = client.split();

    // map the stream
    let stream = stream
        .compat()
        .err_into()
        .try_filter(|m| future::ready(m.is_data()))
        .and_then(|m| {
            if let OwnedMessage::Text(j) = m {
                future::ready(serde_json::from_str::<ServerMessage>(&j).map_err(|e| e.into()))
            } else {
                future::err(format_err!("unexpected message type: {:?}", m))
            }
        });

    // map the sink
    let sink = sink
        .sink_compat()
        .sink_err_into::<failure::Error>()
        .with(|m| {
            future::ready(
                serde_json::to_string(&m)
                    .map_err(|e| e.into())
                    .map(OwnedMessage::Text),
            )
        });

    Ok((sink, stream))
}
