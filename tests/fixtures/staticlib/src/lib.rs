unsafe extern "C" {
    fn host_answer() -> u32;
}

#[unsafe(no_mangle)]
pub extern "C" fn answer() -> u32 {
    unsafe { host_answer() }
}
