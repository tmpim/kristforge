mod krist;
mod miner;
mod network;

use crate::krist::address::Address;
use crate::miner::interface::MinerInterface;
use crate::miner::Target;
use crate::network::{ClientMessage, ServerMessage};
use futures::{future, StreamExt, TryFutureExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::LevelFilter;
use miner::MinerConfig;
use network::{NetConfig, NetworkError};
use std::error::Error;
use std::fs::{create_dir_all, File};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about, author)]
pub enum Opts {
    /// Connect to the krist node and log incoming messages for troubleshooting
    NetLog {
        #[structopt(flatten)]
        net_cfg: NetConfig,
    },

    /// Get information about mining hardware
    Info {},

    /// Mine krist
    Mine {
        #[structopt(flatten)]
        net_cfg: NetConfig,

        #[structopt(flatten)]
        miner_cfg: MinerConfig,

        /// The address to mine krist for
        #[structopt(env = "KRISTFORGE_ADDRESS")]
        address: Address,
    },
}

async fn net_log(net_cfg: NetConfig) -> Result<(), NetworkError> {
    let (_sink, stream) = network::connect(net_cfg).await?;

    println!("Connected!");

    stream
        .try_for_each(|m| {
            println!("{:?}", m);
            future::ok(())
        })
        .await?;

    Ok(())
}

fn system_info() {
    match miner::gpu::get_opencl_devices() {
        Ok(devices) => {
            for d in devices {
                println!("{}", d)
            }
        }
        Err(e) => {
            eprintln!("Error enumerating OpenCL devices: {:?}", e);
        }
    }

    println!("{}", miner::cpu::get_cpu_info());
}

async fn mine(
    net_cfg: NetConfig,
    address: Address,
    miner_cfg: MinerConfig,
) -> Result<(), Box<dyn Error>> {
    let miners = miner::create_miners(miner_cfg)?;

    if miners.is_empty() {
        eprintln!("No miners available!");
        return Ok(());
    }

    let mut target_channels = vec![];
    let (sol_tx, sol_rx) = futures::channel::mpsc::unbounded();

    let multi_pb = MultiProgress::new();

    let wallet_pb = multi_pb.add(ProgressBar::new_spinner());
    wallet_pb.set_style(ProgressStyle::default_bar().template("{wide_msg}"));
    wallet_pb.set_message(&format!("Mining for {}", address));
    let mut mined_kst = 0;

    let target_pb = multi_pb.add(ProgressBar::new_spinner());
    target_pb.set_style(ProgressStyle::default_spinner().template("Current target: {wide_msg}"));

    let miner_style = ProgressStyle::default_spinner().template("{spinner} {prefix}: {wide_msg}");

    for miner in miners {
        let (target_tx, target_rx) = crossbeam::channel::bounded(1);
        target_channels.push(target_tx);

        let name = miner.describe();
        let pb = multi_pb.add(ProgressBar::new_spinner());
        pb.set_prefix(&name);
        pb.set_style(miner_style.clone());
        pb.set_message("Initializing...");

        let interface = MinerInterface::new(address, pb, target_rx, sol_tx.clone());

        std::thread::spawn(move || {
            miner.mine(interface).unwrap();
        });
    }

    std::thread::spawn(move || multi_pb.join().unwrap());

    // set up network connection
    let (sink, stream) = network::connect(net_cfg).await?;

    // set up futures to pipe messages
    let solution_sender = sol_rx
        .map(|n| Ok(ClientMessage::new_solution(address, n)))
        .forward(sink)
        .err_into::<Box<dyn Error>>();

    let target_receiver = stream.err_into().try_for_each(|message| match message {
        ServerMessage::KeepAlive { .. } => future::ok(()),
        ServerMessage::Unknown { msg_type, fields } => {
            log::warn!("Got unknown message type {:?}: {:?}", msg_type, fields);
            future::ok(())
        }
        ServerMessage::Target {
            block,
            work,
            msg_type,
        } => {
            log::info!(
                "Got new mining target - msg_type {} work {} block {:?}",
                msg_type,
                work,
                block
            );

            if address == block.address && msg_type == "response" {
                mined_kst += block.value;
                wallet_pb.set_message(&format!("Mined {} KST for {}", mined_kst, address));
            }

            target_pb.set_message(&format!(
                "Block #{} (shorthash {}, work {})",
                block.height, block.short_hash, work
            ));

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
    });

    future::try_join(solution_sender, target_receiver).await?;

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
        Default::default(),
        File::create(log_file).expect("creating log file"),
    )
    .expect("initializing logger");
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::from_args();

    init_logging();
    log::info!("Arguments: {:?}", opts);

    match opts {
        Opts::NetLog { net_cfg } => {
            if let Err(e) = net_log(net_cfg).await {
                eprintln!("Network error: {:?}", e);
            }
        }
        Opts::Info {} => system_info(),
        Opts::Mine {
            net_cfg,
            address,
            miner_cfg,
        } => {
            if let Err(e) = mine(net_cfg, address, miner_cfg).await {
                eprintln!("Mining error: {:?}", e);
            }
        }
    }
}
