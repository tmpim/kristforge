# kristforge

GPU-accelerated cross platform Krist miner, using OpenCL

## Building

Rust 1.39 is required to build kristforge - [rustup](https://rustup.rs) is the recommended way to install it if you
don't already have it. You will additionally need an OpenCL implementation, usually provided by your graphics card
drivers. Once done, download the repository and, from a terminal in the project directory, run the following command:

```sh
cargo build --release
```

Once complete, the binary can be found at `target/release/kristforge`

## Usage

The complete usage of kristforge can be found by running `kristforge help` and `kristforge help <subcommand>`, but the
basic usage for mining Krist is `kristforge mine <address>`. Unlike previous OpenCL miners, kristforge 2 automatically
adjusts work size to maximize performance.
