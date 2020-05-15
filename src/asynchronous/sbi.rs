#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which)
            : "memory"
            : "volatile");
    }
    ret
}

pub fn set_timer(stime_value: u64) {
    sbi_call(SBI_SET_TIMER, stime_value as usize, 0, 0);
}

const SBI_SET_TIMER: usize = 0;
