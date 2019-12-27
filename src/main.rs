mod krist;
mod network;

use crate::network::{NetConfig, NetworkError};
use futures::{future, TryStreamExt};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum Opts {
    NetLog {
        #[structopt(flatten)]
        net_cfg: NetConfig,
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

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::from_args();

    match opts {
        Opts::NetLog { net_cfg } => {
            if let Err(e) = net_log(net_cfg).await {
                println!("Network error: {:?}", e);
            }
        }
    }
}
