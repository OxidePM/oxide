use anyhow::Result;
use std::path::Path;

// TODO: actually sandbox
#[allow(clippy::unnecessary_wraps)]
pub fn prepare_sandbox<P>(_p: P) -> Result<()>
where
    P: AsRef<Path>,
{
    Ok(())
}
