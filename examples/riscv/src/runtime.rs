use core::alloc::Layout;
use core::panic::PanicInfo;
use core::sync::atomic::*;
use linked_list_allocator::LockedHeap;

pub fn init(cpu_id: usize) {
    static READY: AtomicBool = AtomicBool::new(false);
    if cpu_id == 0 {
        unsafe {
            HEAP_ALLOCATOR
                .lock()
                .init(HEAP.as_ptr() as usize, HEAP_SIZE);
        }
        READY.store(true, Ordering::Release);
    } else {
        while !READY.load(Ordering::Acquire) {
            spin_loop_hint();
        }
    }
}

global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    mv tp, a0

    la sp, bootstack
    sll t0, a0, 14
    add sp, sp, t0

    call rust_main

    .section .bss.stack
    .align 12
    .global bootstack
bootstack:
    .space 4096 * 4 * 4
    .global bootstacktop
bootstacktop:
"#
);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        spin_loop_hint();
    }
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!("abort!");
}

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 0x100000;

static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}