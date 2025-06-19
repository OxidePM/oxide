use crate::{build::BuildArgs, instantiate::InstantiateArgs};
use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    Build(BuildArgs),
    Instantiate(InstantiateArgs),
}
