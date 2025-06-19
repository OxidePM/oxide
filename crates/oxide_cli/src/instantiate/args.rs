use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct InstantiateArgs {
    pub pkg_name: String,
}
