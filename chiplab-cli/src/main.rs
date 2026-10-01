use std::process::ExitCode;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<ExitCode> {
    let _args = Args::parse();

    println!("Hello Chiplab!");

    Ok(ExitCode::SUCCESS)
}
