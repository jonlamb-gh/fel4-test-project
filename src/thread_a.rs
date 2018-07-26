use sel4_sys::{seL4_CPtr, seL4_Call, seL4_MessageInfo_new, seL4_Word};

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub const FAULT_EP_BADGE: seL4_Word = 0x0A;
pub const IPC_EP_BADGE: seL4_Word = 0x1A;

/// arbitrary (but free) address for IPC buffer
pub const IPC_BUFFER_VADDR: seL4_Word = 0x0700_0000;

pub fn run(ep_cap: seL4_CPtr) {
    debug_println!("thread_a::run()");
    debug_println!("thread_a::ep_cap = 0x{:X}", ep_cap,);

    for _ in 0..10 {
        debug_println!("thread_a::sending message to B");

        let msg_info = unsafe { seL4_MessageInfo_new(IPC_EP_BADGE, 0, 0, 0) };

        let _resp_info = unsafe { seL4_Call(ep_cap, msg_info) };
    }

    debug_println!("thread_a::done");
}
