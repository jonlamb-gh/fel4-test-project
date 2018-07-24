use sel4_sys::seL4_Word;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub const FAULT_EP_BADGE: seL4_Word = 0x0A;

pub fn run() {
    debug_println!("thread_a::run()");
}
