use kristforge_core::network::NetOptions;
use structopt::StructOpt;

fn main() {
    let opts = NetOptions::from_args();
    println!("Args: {:?}", opts);
}
