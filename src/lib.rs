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

/// size of thread stack in bytes
const THREAD_STACK_SIZE: usize = 4096;
static mut THREAD_STACK: *const [u64; THREAD_STACK_SIZE / 8] = &[0; THREAD_STACK_SIZE / 8];

/// Returns cap to global fault endpoint if so desired (TODO and handler fn?)
pub fn init(bootinfo: &'static seL4_BootInfo) -> Option<seL4_CPtr> {
    let mut bi_mngr = BootInfoManager::new(bootinfo);

    bi_mngr.debug_print_bootinfo();

    let cspace_cap = seL4_CapInitThreadCNode;
    let pd_cap = seL4_CapInitThreadVSpace;

    let untyped_cap = bi_mngr.get_untyped(None, 1 << seL4_TCBBits).unwrap();
    let tcb_cap = bi_mngr.get_next_free_cap_slot().unwrap();

    let err = bi_mngr.untyped_retype_root(
        untyped_cap,
        api_object_seL4_TCBObject,
        seL4_TCBBits as usize,
        tcb_cap,
    );
    assert!(err == 0, "Failed to retype untyped memory");

    let err: seL4_Error = unsafe {
        seL4_TCB_Configure(
            tcb_cap,
            seL4_CapNull.into(), // fault_ep
            cspace_cap.into(),
            seL4_NilData.into(),
            pd_cap.into(),
            seL4_NilData.into(),
            0, //ipc_buffer_vaddr,
            0, //ipc_frame_cap,
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

    None
}

fn run() {
    debug_println!("lib::run()");
}
