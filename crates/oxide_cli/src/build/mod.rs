mod args;
pub use args::*;

use anyhow::{Result, bail};
use oxide_pkgs::top_level::all_packages::all_pkgs;
use oxide_store::{api::Store, build::build, instantiate::instantiate, stores::local::LocalStore};

type S = LocalStore;

pub async fn build_cli(args: BuildArgs) -> Result<()> {
    if let Some(pkg_name) = args.path.strip_prefix("oxide#") {
        let (pkgs, _) = all_pkgs();

        if let Some(pkg) = pkgs.get(pkg_name) {
            let store = S::new().await?;
            let (_, path) = instantiate(&store, pkg).await?;
            let outputs = build(&store, &path).await?;
            for (out, p) in outputs {
                println!("{}!{}", S::store_path(&p), out);
            }
        } else {
            bail!("pkg {} not found", pkg_name);
        }
    } else {
        // TODO: build derivations outside the pkgs collection
        bail!(
            "Derivations outside the pkgs collection are not yet supported. Preappend the path with oxide#pkg_name"
        )
    }

    Ok(())
}
