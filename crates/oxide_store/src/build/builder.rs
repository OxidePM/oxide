use crate::{api::Store, os::sandbox::prepare_sandbox, utils::tempfile::tempdir_in};
use anyhow::{Result, bail};
use oxide_core::drv::StoreDrv;
use std::{collections::HashMap, ffi::CString, ptr};

pub const SANDBOX_BUILD_DIR: &str = "/build";

// TODO: maybe rewrites must be passed here and be used somewhere outside of build
pub async fn run_builder<S>(drv: &StoreDrv) -> Result<()>
where
    S: Store,
{
    prepare_build::<S>(drv).await?;
    Ok(())
}

// TODO: maybe replace envs with Cows since many of them are &str
// this reduces the number of allocations by a lot
fn builder_envs<S>(drv: &StoreDrv) -> HashMap<String, String>
where
    S: Store,
{
    let mut envs = HashMap::new();
    envs.insert("PATH".to_string(), "/path-not-set".to_string());
    envs.insert("HOME".to_string(), "/homeless-shelter".to_string());
    envs.insert("OXIDE_STORE".to_string(), S::store_dir());
    // store derivation envs
    envs.extend(drv.envs.iter().map(|(k, v)| (k.clone(), v.clone())));

    envs.insert("TMPDIR".to_string(), SANDBOX_BUILD_DIR.to_string());
    envs.insert("TEMPDIR".to_string(), SANDBOX_BUILD_DIR.to_string());
    envs.insert("TMP".to_string(), SANDBOX_BUILD_DIR.to_string());
    envs.insert("TEMP".to_string(), SANDBOX_BUILD_DIR.to_string());

    envs.insert("TERM".to_string(), "xterm-256color".to_string());
    envs
}

async fn prepare_build<S>(drv: &StoreDrv) -> Result<()>
where
    S: Store,
{
    let top_tmp_dir = tempdir_in(S::store_dir()).await?;
    // skip the slash :)
    let tmp_dir = top_tmp_dir.join(&SANDBOX_BUILD_DIR[1..]);
    let envs = builder_envs::<S>(&drv);
    prepare_sandbox(&tmp_dir)?;

    unsafe {
        run_process(&drv, envs)?;
    }

    Ok(())
}

fn strings_to_charptr(strs: Vec<String>) -> Result<(Vec<CString>, Vec<*const libc::c_char>)> {
    let cstrings = strs
        .into_iter()
        .map(|s| CString::new(s))
        .collect::<Result<Vec<CString>, _>>()?;
    let mut charptr = cstrings
        .iter()
        .map(|s| s.as_ptr() as *const libc::c_char)
        .collect::<Vec<_>>();
    charptr.push(ptr::null());
    Ok((cstrings, charptr))
}

fn run_child(drv: &StoreDrv, envs: HashMap<String, String>) -> Result<()> {
    let mut args = Vec::new();
    args.push(drv.builder.to_string());
    for arg in drv.args.iter() {
        args.push(arg.clone());
    }
    let mut env_strs = Vec::new();
    for (k, v) in envs.iter() {
        env_strs.push(format!("{}={}", k, v));
    }
    exec_builder(&drv.builder, args, env_strs)?;
    Ok(())
}

fn exec_builder(builder: &str, args: Vec<String>, envs: Vec<String>) -> Result<()> {
    // do not remove _args and _envs otherwise they might get dropped
    // and the pointers will point to dirty memory
    let (_args, args) = strings_to_charptr(args)?;
    let (_envs, envs) = strings_to_charptr(envs)?;
    let builder = CString::new(builder)?;
    let builder = builder.as_ptr() as *const libc::c_char;
    unsafe {
        let code = libc::execve(builder, args.as_ptr(), envs.as_ptr());
        if code == -1 {
            libc::exit(1);
        }
    }
    Ok(())
}

unsafe fn run_process(drv: &StoreDrv, envs: HashMap<String, String>) -> Result<()> {
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        run_child(&drv, envs)
    } else if pid == -1 {
        bail!("unable to fork process");
    } else {
        let mut status = 0 as libc::c_int;
        unsafe {
            libc::waitpid(pid, &mut status, 0);
        }
        Ok(())
    }
}
