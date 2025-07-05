// TODO: remove :)
#[allow(dead_code)]
pub fn errno() -> libc::c_int {
    unsafe { *libc::__errno_location() }
}
