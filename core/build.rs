fn main() {
    // pass through TARGET env var so it can be used in the user agent
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
