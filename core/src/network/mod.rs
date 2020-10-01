use crate::krist::{ServerMessage, Solution};
use color_eyre::eyre::{self, WrapErr};
use color_eyre::Help;
use futures_util::future::ready;
use futures_util::sink::Sink;
use futures_util::stream::Stream;
use futures_util::{SinkExt, TryFutureExt, TryStreamExt};
use reqwest::header::USER_AGENT;
use reqwest::Client;
use serde::Deserialize;
use std::fmt::Debug;
use tracing::info;

/// Hacky convenience macro to simplify getting default values for options.
macro_rules! default {
    (@node) => {
        "https://krist.ceriat.net/ws/start"
    };
    (@ua) => {
        concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION"),
            " (",
            env!("TARGET"),
            ")"
        )
    };
}

/// Options for connecting to the krist node.
#[derive(Debug, Deserialize)]
#[serde(default)]
#[cfg_attr(not(target_arch = "wasm32"), derive(structopt::StructOpt))]
pub struct NetOptions {
    /// The URL for the websocket initiation endpoint of the krist node.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        structopt(short, long, default_value = default!(@node))
    )]
    node: String,

    /// The user agent to report to the krist node.
    #[cfg_attr(not(target_arch = "wasm32"), structopt(long, default_value = default!(@ua)))]
    user_agent: String,
}

impl Default for NetOptions {
    fn default() -> Self {
        Self {
            node: String::from(default!(@node)),
            user_agent: String::from(default!(@ua)),
        }
    }
}

/// Establish a connection to the krist node using the given options.
#[tracing::instrument(err)]
pub async fn connect_krist(
    NetOptions { node, user_agent }: NetOptions,
) -> eyre::Result<(
    impl Sink<Solution, Error = eyre::Error>,
    impl Stream<Item = eyre::Result<ServerMessage>>,
)> {
    #[derive(Debug, Deserialize)]
    struct WsToken {
        url: String,
    }

    // request websocket session
    info!("Requesting websocket url");
    let WsToken { url } = Client::new()
        .post(&node)
        .header(USER_AGENT, &user_agent)
        .send()
        .and_then(|r| r.json())
        .await
        .wrap_err("websocket session request failed")
        .suggestion("is the krist node URL correct?")
        .suggestion("is your internet connection working?")?;

    // open connection using received url
    info!(%url, "Starting websocket connection");

    #[cfg(not(target_arch = "wasm32"))]
    let con = {
        use async_tungstenite::tungstenite::http::header::USER_AGENT;
        use async_tungstenite::tungstenite::http::Request;
        use async_tungstenite::tungstenite::Message;
        use futures_util::future::ok;
        use futures_util::StreamExt;

        let req = Request::builder()
            .uri(url)
            .header(USER_AGENT, &user_agent)
            .body(())
            .unwrap();

        let (ws, res) = async_tungstenite::tokio::connect_async(req)
            .await
            .wrap_err("websocket connection failed")
            .suggestion("is your internet connection working?")?;

        info!(?res, "Websocket connection established");

        ws.map_err(|e| eyre::Report::new(e).wrap_err("receiving server message"))
            .try_filter_map(|m| ok(m.into_text().ok()))
            .and_then(|t| ready(ServerMessage::from_json(t)))
            .sink_map_err(|e| eyre::Report::new(e).wrap_err("sending submission"))
            .with(|s: Solution| ready(Ok(Message::Text(s.to_json()))))
            .split()
    };

    #[cfg(target_arch = "wasm32")]
    let con = {
        use futures_util::stream::select;
        use futures_util::StreamExt;
        use pharos::{Observable, ObserveConfig};
        use ws_stream_wasm::{WsEvent, WsMessage, WsMeta};

        let (mut ws_meta, ws) = WsMeta::connect(url, None)
            .await
            .wrap_err("websocket connection failed")
            .suggestion("is your internet connection working?")?;

        let errs = ws_meta
            .observe(ObserveConfig::default())
            .await
            .wrap_err("error initializing observer")?
            .filter_map(|e| {
                ready(match e {
                    WsEvent::Error => Some(eyre::eyre!("unspecified connection error")),
                    WsEvent::Closed(e) => Some(eyre::eyre!("websocket closed: {:?}", e)),
                    WsEvent::WsErr(e) => Some(eyre::Report::new(e)),
                    _ => None,
                })
            })
            .map(Err);

        let (sink, stream) = ws.split();

        let stream = select(
            stream.map(|m| {
                String::from_utf8(m.as_ref().to_vec())
                    .wrap_err_with(|| format!("invalid utf8 from server: {:?}", m))
            }),
            errs,
        )
        .and_then(|t| ready(ServerMessage::from_json(t)));

        let sink = sink
            .sink_map_err(|e| eyre::Report::new(e).wrap_err("sending submission"))
            .with(|s: Solution| ready(Ok(WsMessage::Text(s.to_json()))));

        (sink, stream)
    };

    Ok(con)
}
