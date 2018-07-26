use sel4_sys::{seL4_CPtr, seL4_MessageInfo_new, seL4_Recv, seL4_Reply, seL4_Word};

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub const FAULT_EP_BADGE: seL4_Word = 0x0B;
pub const IPC_EP_BADGE: seL4_Word = 0x1B;

/// arbitrary (but free) address for IPC buffer
pub const IPC_BUFFER_VADDR: seL4_Word = 0x0700_1000;

pub fn run(ep_cap: seL4_CPtr, aux_ep_cap: seL4_CPtr) {
    debug_println!("thread_b::run()");
    debug_println!(
        "thread_b::ep_cap = 0x{:X} - aux_ep_cap = 0x{:X}",
        ep_cap,
        aux_ep_cap
    );

    for _ in 0..10 {
        let mut badge: seL4_Word = 0;
        let _msg_info = unsafe { seL4_Recv(ep_cap, &mut badge) };

        debug_println!("thread_b::got msg from A, sending reply");

        let resp_info = unsafe { seL4_MessageInfo_new(IPC_EP_BADGE, 0, 0, 0) };

        unsafe { seL4_Reply(resp_info) };
    }

    debug_println!("thread_b::done");
}
