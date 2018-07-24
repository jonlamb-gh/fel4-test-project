use sel4_sys::seL4_Word;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub const FAULT_EP_BADGE: seL4_Word = 0x0B;

/// arbitrary (but free) address for IPC buffer
pub const IPC_BUFFER_VADDR: seL4_Word = 0x0700_1000;

pub fn run() {
    debug_println!("thread_b::run()");
}
