#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]

extern crate alloc;
extern crate sel4_sys;

#[macro_use]
mod macros;
mod bootinfo_manager;
mod thread_a;
mod thread_b;

use alloc::boxed::Box;
use bootinfo_manager::BootInfoManager;
use core::mem;
use sel4_sys::*;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

/// size of thread stack in bytes
const THREAD_STACK_SIZE: usize = 4096;

pub struct InitSystem {
    bi_mngr: BootInfoManager,
}

impl InitSystem {
    pub fn new(bootinfo: &'static seL4_BootInfo) -> InitSystem {
        InitSystem {
            bi_mngr: BootInfoManager::new(bootinfo),
        }
    }

    /// Returns cap to global fault endpoint if one is created/used
    pub fn init(&mut self) -> Option<seL4_CPtr> {
        self.bi_mngr.debug_print_bootinfo();

        let global_fault_ep_cap = self.create_global_fault_ep();

        self.create_thread(
            global_fault_ep_cap,
            thread_a::FAULT_EP_BADGE,
            thread_a::IPC_BUFFER_VADDR,
            thread_a::run,
        );

        self.create_thread(
            global_fault_ep_cap,
            thread_b::FAULT_EP_BADGE,
            thread_b::IPC_BUFFER_VADDR,
            thread_b::run,
        );

        Some(global_fault_ep_cap)
    }

    pub fn is_fault(&self, badge: seL4_Word) -> bool {
        match badge {
            thread_a::FAULT_EP_BADGE => true,
            thread_b::FAULT_EP_BADGE => true,
            _ => false,
        }
    }

    pub fn handle_fault(&self, badge: seL4_Word) {
        debug_println!("!!! thread faulted - badge = 0x{:X} !!!\n", badge);
    }

    fn create_global_fault_ep(&mut self) -> seL4_CPtr {
        let untyped_cap = self
            .bi_mngr
            .get_untyped(None, 1 << seL4_EndpointBits)
            .unwrap();

        let ep_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();

        let err = self.bi_mngr.untyped_retype_root(
            untyped_cap,
            api_object_seL4_EndpointObject,
            seL4_EndpointBits as _,
            ep_cap,
        );
        assert!(err == 0, "Failed to retype untyped memory");

        ep_cap
    }

    fn create_thread(
        &mut self,
        fault_ep_cap: seL4_CPtr,
        fault_ep_badge: seL4_Word,
        ipc_buffer_vaddr: seL4_Word,
        run_fn: fn(),
    ) {
        let cspace_cap = seL4_CapInitThreadCNode;
        let pd_cap = seL4_CapInitThreadVSpace;

        // untyped large enough for:
        // - thread TCB
        // - IPC frame
        // - badged endpoint (provided to thread as fault ep)
        let untyped_size_bytes =
            (1 << seL4_TCBBits) + (1 << seL4_PageBits) + (1 << seL4_EndpointBits);

        let untyped_cap = self.bi_mngr.get_untyped(None, untyped_size_bytes).unwrap();

        let tcb_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();
        let ipc_frame_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();
        let badged_ep_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();

        let err = self.bi_mngr.untyped_retype_root(
            untyped_cap,
            api_object_seL4_TCBObject,
            seL4_TCBBits as _,
            tcb_cap,
        );
        assert!(err == 0, "Failed to retype untyped memory");

        let err = self.bi_mngr.untyped_retype_root(
            untyped_cap,
            _object_seL4_ARM_SmallPageObject,
            seL4_PageBits as _,
            ipc_frame_cap,
        );
        assert!(err == 0, "Failed to retype untyped memory");

        // map the frame into the vspace at ipc_buffer_vaddr
        let err = self
            .bi_mngr
            .map_paddr(untyped_cap, ipc_frame_cap, ipc_buffer_vaddr);
        assert!(err == 0, "Failed to map IPC frame");

        // set the IPC buffer's virtual address in a field of the IPC buffer
        let ipc_buffer: *mut seL4_IPCBuffer = ipc_buffer_vaddr as _;
        unsafe { (*ipc_buffer).userData = ipc_buffer_vaddr };

        // mint a copy of the endpoint cap into our cspace
        let err: seL4_Error = unsafe {
            seL4_CNode_Mint(
                cspace_cap,
                badged_ep_cap,
                seL4_WordBits as _,
                cspace_cap,
                fault_ep_cap,
                seL4_WordBits as _,
                seL4_CapRights_new(1, 1, 1),
                fault_ep_badge,
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
                ipc_buffer_vaddr,
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

        let thread_stack_box: Box<u64> = Box::new((THREAD_STACK_SIZE / 8) as u64);
        let stack_base: &'static mut u64 = Box::leak(thread_stack_box);
        let stack_top = stack_base as *const _ as usize + THREAD_STACK_SIZE;

        assert!(
            stack_top % stack_alignment_requirement == 0,
            "Thread stack is not properly aligned to a {} byte boundary",
            stack_alignment_requirement
        );

        let mut regs: seL4_UserContext = unsafe { mem::zeroed() };

        #[allow(const_err)]
        {
            regs.pc = run_fn as _;
        }

        regs.sp = stack_top as seL4_Word;

        let context_size = 2;
        let err = unsafe { seL4_TCB_WriteRegisters(tcb_cap, 0, 0, context_size, &mut regs) };
        assert!(err == 0, "Failed to write TCB registers");

        let err = unsafe { seL4_TCB_SetPriority(tcb_cap, seL4_CapInitThreadTCB.into(), 255) };
        assert!(err == 0, "Failed to set TCB priority");

        let err = unsafe { seL4_TCB_Resume(tcb_cap) };
        assert!(err == 0, "Failed to resume TCB");
    }
}
