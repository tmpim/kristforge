use color_eyre::eyre;
use futures_util::TryStreamExt;
use kristforge_core::network::{connect_krist_raw, NetOptions};
use tracing::{debug, info};
use tracing_error::ErrorLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_wasm::WASMLayer;
use wasm_bindgen::prelude::*;

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::new("debug"))
        .with(ErrorLayer::default())
        .with(WASMLayer::default())
        .init();
}

#[tracing::instrument(err)]
async fn net_logger(options: NetOptions) -> eyre::Result<()> {
    let (_, mut rx) = connect_krist_raw(options).await?;

    while let Some(msg) = rx.try_next().await? {
        debug!(%msg)
    }

    Ok(())
}

#[wasm_bindgen]
pub async fn run_net_logger(options: JsValue) -> Result<(), JsValue> {
    let options = if options.is_null() || options.is_undefined() {
        NetOptions::default()
    } else {
        options.into_serde().unwrap()
    };

    net_logger(options)
        .await
        .map_err(|e| JsValue::from(format!("{:?}", e)))
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    init_tracing();
    info!(
        "Initializing {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    Ok(())
}
