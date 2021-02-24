mod thread_priority;

use color_eyre::eyre::{self, eyre, WrapErr};
use futures_util::TryStreamExt;
use kristforge_core::network::{connect_krist_raw, NetOptions};
use std::fs::{create_dir_all, File};
use std::io::Write;
use structopt::StructOpt;
use tokio::runtime;
use tracing::{debug, info};
use tracing_error::ErrorLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Connect to the krist node and log events, for debugging purposes.
    NetLog {
        #[structopt(flatten)]
        net_opts: NetOptions,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub struct Options {
    #[structopt(flatten)]
    command: Command,

    /// Don't lower worker thread priority (may hog resources when CPU mining)
    #[structopt(long)]
    keep_thread_priority: bool,
}

fn open_log_file() -> eyre::Result<impl Write> {
    let data_dir = dirs_next::data_dir()
        .ok_or_else(|| eyre!("couldn't determine data directory"))?
        .join(env!("CARGO_BIN_NAME"));
    create_dir_all(&data_dir).wrap_err("couldn't create data dir")?;
    File::create(data_dir.join(concat!(env!("CARGO_BIN_NAME"), ".log")))
        .wrap_err("couldn't open log file")
}

fn init_tracing() -> eyre::Result<impl Drop> {
    let (log, guard) = tracing_appender::non_blocking(open_log_file()?);

    let filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(log);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    Ok(guard)
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let _log_guard = init_tracing()?;

    let opts = Options::from_args();
    info!(
        ?opts,
        "Starting {} v{}",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let mut rt = runtime::Builder::new_multi_thread();
    rt.enable_all();
    if !opts.keep_thread_priority {
        rt.on_thread_start(thread_priority::deprioritize_thread);
    }
    let rt = rt.build().wrap_err("couldn't create runtime")?;

    rt.block_on(run(opts))
}

#[tracing::instrument(err)]
async fn run(opts: Options) -> eyre::Result<()> {
    match opts.command {
        Command::NetLog { net_opts } => {
            let (_, mut rx) = connect_krist_raw(net_opts).await?;
            eprintln!("Websocket connection established");

            while let Some(msg) = rx.try_next().await? {
                debug!(%msg);
                println!("{}", msg);
            }

            eprintln!("Websocket connection closed");
        }
    }

    Ok(())
}
