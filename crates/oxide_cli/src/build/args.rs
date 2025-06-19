use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct BuildArgs {
    pub path: String,
}
