pub fn errno() -> libc::c_int {
    return unsafe { *libc::__errno_location() };
}
