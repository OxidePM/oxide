use anyhow::{Result, bail};
use std::{ffi::CString, mem, os::unix::ffi::OsStrExt, path::Path};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(dead_code)] // TODO: locks :)
pub enum LockMode {
    Read,
    Write,
    UnLock,
}

// TODO: this is very sketchy
// check validity of this code
pub struct PathLock {
    fd: libc::c_int,
    path: CString,
}

impl PathLock {
    pub fn lock<P>(p: P, mode: LockMode) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let p = p.as_ref();
        loop {
            let (fd, path) = PathLock::open_lock(p)?;
            let mode = match mode {
                LockMode::Read => libc::LOCK_SH,
                LockMode::Write => libc::LOCK_EX,
                LockMode::UnLock => libc::LOCK_UN,
            };
            if unsafe { libc::flock(fd, mode) } != 0 {
                bail!("could not lock file {:?}", p)
            }
            let mut stat = mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
                bail!("cloud not fstat lock file {:?}:", p)
            }
            let stat = unsafe { stat.assume_init() };
            if stat.st_size != 0 {
                // stale lock
                continue;
            }
            return Ok(PathLock { fd, path });
        }
    }

    pub fn open_lock<P>(p: P) -> Result<(libc::c_int, CString)>
    where
        P: AsRef<Path>,
    {
        let p = p.as_ref();
        let raw_path = p.as_os_str().as_bytes();
        let path = CString::new(raw_path)?;
        let fd = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_CLOEXEC | libc::O_RDWR | libc::O_CREAT,
                666,
            )
        };
        if fd == 0 {
            bail!("could not open lock file {:?}", p)
        }
        Ok((fd, path))
    }

    #[allow(clippy::unused_self)]
    #[inline]
    /// unlocks by running the distructor
    pub fn unlock(self) {}

    fn unlock_ref(&self) {
        unsafe {
            libc::unlink(self.path.as_ptr());
            libc::write(self.fd, b" ".as_ptr().cast::<libc::c_void>(), 1);
            libc::close(self.fd);
        }
    }
}

impl Drop for PathLock {
    fn drop(&mut self) {
        self.unlock_ref();
    }
}
