mod args;
mod build;
mod instantiate;
mod logger;

use anyhow::Result;
use args::{Args, Command};
use build::build_cli;
use clap::Parser;
use instantiate::instantiate_cli;
use log::LevelFilter;
use logger::Logger;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { cli().await })
}

fn setup_logging() {
    if log::set_logger(&Logger).is_err() {
        eprintln!("Unable to set logger, proceeding without one");
    } else {
        log::set_max_level(LevelFilter::Info);
    }
}

async fn cli() -> Result<()> {
    let args = Args::parse();
    if args.verbose {
        setup_logging();
    }
    match args.command {
        Command::Build(args) => build_cli(args).await,
        Command::Instantiate(args) => instantiate_cli(args).await,
    }
}
