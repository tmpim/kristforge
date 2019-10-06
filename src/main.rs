pub mod ext;
pub mod krist;
pub mod miner;
pub mod network;
pub mod prelude;

use crate::miner::types::Target;
use crate::network::types::{ClientMessage, ServerMessage};
use ext::DeviceExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::LevelFilter;
use miner::interface::MinerInterface;
use miner::miner::Miner;
use miner::selector::Selector;
use miner::types::MinerConfig;
use network::types::NetConfig;
use prelude::*;
use simplelog::Config;
use std::fs::{create_dir_all, File};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub enum Opts {
    /// Connect to the krist node and log incoming messages without mining
    NetLog {
        #[structopt(flatten)]
        cfg: NetConfig,
    },

    /// Show hardware information
    Info,

    /// Mine for krist
    Mine {
        #[structopt(flatten)]
        net_cfg: NetConfig,

        #[structopt(flatten)]
        miner_cfg: MinerConfig,

        /// Skip miner tests and begin mining immediately
        #[structopt(short, long)]
        skip_tests: bool,
    },
}

async fn net_logger(cfg: &NetConfig) -> Fallible<()> {
    eprintln!("Connecting using {:?}", cfg);
    let (_sink, stream) = network::websocket::connect(cfg).await?;

    stream
        .try_for_each(|m| {
            eprintln!("Got message: {:?}", m);
            future::ok(())
        })
        .await?;

    Ok(())
}

fn init_logging() {
    let log_file = dirs::data_dir()
        .map(|d| d.join(env!("CARGO_PKG_NAME")))
        .unwrap_or_default()
        .join(concat!(env!("CARGO_PKG_NAME"), ".log"));

    if let Some(parent) = log_file.parent() {
        create_dir_all(parent).expect("creating data dirs");
    }

    simplelog::WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create(log_file).expect("creating log file"),
    )
    .expect("initializing logger");
}

fn main() -> Fallible<()> {
    use Opts::*;
    let opts: Opts = StructOpt::from_args();

    init_logging();

    match opts {
        NetLog { cfg } => {
            let rt = Runtime::new()?;
            rt.block_on(net_logger(&cfg))?;
        }
        Info => {
            let mut tree = String::new();
            ascii_tree::write_tree(&mut tree, &Selector::ascii_tree()?)?;
            eprintln!("Available mining hardware:\n{}", tree);
        }
        Mine {
            miner_cfg,
            net_cfg,
            skip_tests,
        } => {
            let devices = Selector::select_all(&miner_cfg.devices)?;

            debug!(
                "Selected devices: {:?}",
                devices
                    .iter()
                    .map(|d| d.human_name())
                    .collect::<Result<Vec<_>, _>>()?
            );

            eprintln!("Initializing miners on {} device(s)...", devices.len());

            // initialize the miners
            let miners: Vec<Miner> = devices
                .into_iter()
                .map(|d| Miner::init(d, &miner_cfg))
                .collect::<Result<_, _>>()?;

            // run tests
            if !skip_tests {
                info!("Running tests");
                for m in &miners {
                    m.test()?;
                }
            }

            crossbeam::scope(|s| -> Fallible<()> {
                let mut target_channels = vec![];
                let (sol_tx, sol_rx) = tokio::sync::mpsc::unbounded_channel();

                let multi_pb = MultiProgress::new();

                for miner in miners {
                    let name = miner.pq().device().human_name()?;
                    let (target_tx, target_rx) = crossbeam::channel::bounded(1);
                    target_channels.push(target_tx);

                    let pb = multi_pb.add(ProgressBar::new_spinner());
                    pb.set_prefix(&name);
                    pb.set_style(
                        ProgressStyle::default_spinner().template("{spinner} {prefix}: {wide_msg}"),
                    );

                    let interface =
                        MinerInterface::new(rand::random(), target_rx, pb, sol_tx.clone());

                    s.builder()
                        .name(format!("Miner on {}", name))
                        .spawn(move |_| {
                            miner.start_miner(interface).unwrap();
                        })?;
                }

                s.spawn(move |_| multi_pb.join().unwrap());

                // set up network connection
                let rt = Runtime::new()?;
                let (sink, stream) = rt.block_on(network::websocket::connect(&net_cfg))?;

                // set up futures to pipe messages
                let solution_sender = sol_rx.map(|s| Ok(ClientMessage::from(s))).forward(sink);
                let target_receiver = stream.try_for_each(|message: ServerMessage| match message {
                    ServerMessage::Target {
                        block,
                        work,
                        msg_type,
                    } => {
                        debug!(
                            "Got new mining target - type {} work {} block {:?}",
                            msg_type, work, &block
                        );

                        for tx in &target_channels {
                            if let Err(e) = tx.send(Target {
                                block: block.short_hash,
                                work,
                            }) {
                                return future::err(e.into());
                            }
                        }

                        future::ok(())
                    }
                    ServerMessage::KeepAlive { .. } => future::ok(()),
                    ServerMessage::Unknown { msg_type, fields } => {
                        debug!("Got unknown message type {:?}: {:?}", msg_type, fields);
                        future::ok(())
                    }
                });

                // run the futures
                rt.block_on(future::try_join(solution_sender, target_receiver))?;

                Ok(())
            })
            .unwrap()?;
        }
    }

    Ok(())
}
