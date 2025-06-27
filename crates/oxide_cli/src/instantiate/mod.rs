use anyhow::{Result, bail};

mod args;
pub use args::*;
use oxide_pkgs::top_level::all_packages::all_pkgs;
use oxide_store::{api::Store, instantiate::instantiate, stores::local::LocalStore};

type S = LocalStore;

pub async fn instantiate_cli(args: InstantiateArgs) -> Result<()> {
    let (pkgs, _) = all_pkgs();

    if let Some(pkg) = pkgs.get(&args.pkg_name) {
        let store = S::new().await?;
        let (_, p) = instantiate(&store, pkg).await?;
        println!("{}", S::store_path(&p));
    } else {
        bail!("pkg {} not found", args.pkg_name);
    }

    Ok(())
}
