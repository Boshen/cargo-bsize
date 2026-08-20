unsafe extern "C" {
    fn host_answer() -> u32;
}

#[unsafe(no_mangle)]
pub extern "C" fn answer() -> u32 {
    // SAFETY: the host that links this archive provides `host_answer`.
    unsafe { host_answer() }
}
