#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]

#[cfg(all(feature = "alloc"))]
#[macro_use]
extern crate alloc;

#[cfg(all(feature = "test"))]
#[macro_use]
extern crate proptest;

extern crate sel4_sys;

#[cfg(feature = "test")]
pub mod fel4_test;

#[macro_use]
mod macros;
mod bootinfo_manager;

use bootinfo_manager::BootInfoManager;
use core::mem;
use sel4_sys::*;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub const FAULT_EP_BADGE: seL4_Word = 0x61;

/// size of thread stack in bytes
const THREAD_STACK_SIZE: usize = 4096;
static mut THREAD_STACK: *const [u64; THREAD_STACK_SIZE / 8] = &[0; THREAD_STACK_SIZE / 8];

/// arbitrary (but free) address for IPC buffer
const IPC_BUFFER_VADDR: seL4_Word = 0x0700_0000;

/// Returns cap to global fault endpoint if so desired (TODO and handler fn?)
pub fn init(bootinfo: &'static seL4_BootInfo) -> Option<seL4_CPtr> {
    let mut bi_mngr = BootInfoManager::new(bootinfo);

    bi_mngr.debug_print_bootinfo();

    let cspace_cap = seL4_CapInitThreadCNode;
    let pd_cap = seL4_CapInitThreadVSpace;

    // untyped large enough for:
    // - thread TCB
    // - IPC frame
    // - endpoint (global fault ep)
    // - badged endpoint (provided to thread as fault ep)
    let untyped_size_bytes = (1 << seL4_TCBBits) + (1 << seL4_PageBits) + (1 << seL4_EndpointBits);

    let untyped_cap = bi_mngr.get_untyped(None, untyped_size_bytes).unwrap();

    // TODO - should IPC cap use get_frame_cap() / io_map()?
    let tcb_cap = bi_mngr.get_next_free_cap_slot().unwrap();
    let ipc_frame_cap = bi_mngr.get_next_free_cap_slot().unwrap();
    let ep_cap = bi_mngr.get_next_free_cap_slot().unwrap();
    let badged_ep_cap = bi_mngr.get_next_free_cap_slot().unwrap();

    let err = bi_mngr.untyped_retype_root(
        untyped_cap,
        api_object_seL4_TCBObject,
        seL4_TCBBits as _,
        tcb_cap,
    );
    assert!(err == 0, "Failed to retype untyped memory");

    let err = bi_mngr.untyped_retype_root(
        untyped_cap,
        _object_seL4_ARM_SmallPageObject,
        seL4_PageBits as _,
        ipc_frame_cap,
    );
    assert!(err == 0, "Failed to retype untyped memory");

    let err = bi_mngr.untyped_retype_root(
        untyped_cap,
        api_object_seL4_EndpointObject,
        seL4_EndpointBits as _,
        ep_cap,
    );
    assert!(err == 0, "Failed to retype untyped memory");

    // map the frame into the vspace at ipc_buffer_vaddr
    let err = bi_mngr.map_paddr(untyped_cap, ipc_frame_cap, IPC_BUFFER_VADDR);
    assert!(err == 0, "Failed to map IPC frame");

    // set the IPC buffer's virtual address in a field of the IPC buffer
    let mut ipc_buffer: *mut seL4_IPCBuffer = IPC_BUFFER_VADDR as _;
    unsafe { (*ipc_buffer).userData = IPC_BUFFER_VADDR };

    // Mint a copy of the endpoint cap into our cspace
    let err: seL4_Error = unsafe {
        seL4_CNode_Mint(
            cspace_cap,
            badged_ep_cap,
            seL4_WordBits as _,
            cspace_cap,
            ep_cap,
            seL4_WordBits as _,
            seL4_CapRights_new(1, 1, 1),
            0x61, // badge
        )
    };
    assert!(err == 0, "Failed to mint a copy of the fault endpoint");

    let err: seL4_Error = unsafe {
        seL4_TCB_Configure(
            tcb_cap,
            badged_ep_cap,
            cspace_cap.into(),
            seL4_NilData.into(),
            pd_cap.into(),
            seL4_NilData.into(),
            IPC_BUFFER_VADDR,
            ipc_frame_cap,
        )
    };
    assert!(err == 0, "Failed to configure TCB");

    let stack_alignment_requirement: usize = (seL4_WordBits as usize / 8) * 2;

    assert!(THREAD_STACK_SIZE >= 512, "Thread stack size is too small");
    assert!(
        THREAD_STACK_SIZE % stack_alignment_requirement == 0,
        "Thread stack is not properly aligned to a {} byte boundary",
        stack_alignment_requirement
    );

    let stack_base = unsafe { THREAD_STACK as usize };
    let stack_top = stack_base + THREAD_STACK_SIZE;

    assert!(
        stack_top % stack_alignment_requirement == 0,
        "Thread stack is not properly aligned to a {} byte boundary",
        stack_alignment_requirement
    );

    let mut regs: seL4_UserContext = unsafe { mem::zeroed() };

    #[allow(const_err)]
    {
        regs.pc = run as _;
    }

    regs.sp = stack_top as seL4_Word;

    let context_size = 2;
    let err = unsafe { seL4_TCB_WriteRegisters(tcb_cap, 0, 0, context_size, &mut regs) };
    assert!(err == 0, "Failed to write TCB registers");

    let err = unsafe { seL4_TCB_SetPriority(tcb_cap, seL4_CapInitThreadTCB.into(), 255) };
    assert!(err == 0, "Failed to set TCB priority");

    let err = unsafe { seL4_TCB_Resume(tcb_cap) };
    assert!(err == 0, "Failed to resume TCB");

    Some(ep_cap)
}

fn run() {
    debug_println!("lib::run()");
    debug_println!("lib::run() about to fault");
}
