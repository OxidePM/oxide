use anyhow::Result;
use oxide_instantiate::instantiate;
use oxide_pkgs::top_level::all_packages;

#[tokio::main]
async fn main() -> Result<()> {
    let pkgs = all_packages::all_pkgs();
    instantiate(&pkgs.hello).await?;
    instantiate(&pkgs.perl).await?;
    Ok(())
}
