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
use alloc::vec::Vec;
use bootinfo_manager::BootInfoManager;
use core::mem;
use sel4_sys::*;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

/// size of thread stack in bytes
const THREAD_STACK_SIZE: usize = 4096;

struct ThreadInfo {
    tcb_cap: seL4_CPtr,
    fault_ep_cap: seL4_CPtr,
    fault_ep_badge: seL4_Word,
    ipc_ep_cap: seL4_CPtr,
    ipc_ep_badge: seL4_Word,
}

pub struct InitSystem {
    bi_mngr: BootInfoManager,
    thread_infos: Vec<ThreadInfo>,
}

impl InitSystem {
    /// This will be created from the callers's (root-task) stack
    pub fn new(bootinfo: &'static seL4_BootInfo) -> InitSystem {
        InitSystem {
            bi_mngr: BootInfoManager::new(bootinfo),
            thread_infos: Vec::new(),
        }
    }

    /// Returns cap to global fault endpoint if one is created/used
    pub fn init(&mut self) -> Option<seL4_CPtr> {
        self.bi_mngr.debug_print_bootinfo();

        let global_fault_ep_cap = self.create_ep();

        self.create_thread(
            global_fault_ep_cap,
            thread_b::FAULT_EP_BADGE,
            thread_b::IPC_EP_BADGE,
            thread_b::IPC_BUFFER_VADDR,
            None,
            thread_b::run,
        );

        let thread_b_ipc_ep_cap = self
            .thread_infos
            .iter()
            .find(|t| t.ipc_ep_badge == thread_b::IPC_EP_BADGE)
            .unwrap()
            .ipc_ep_cap;

        self.create_thread(
            global_fault_ep_cap,
            thread_a::FAULT_EP_BADGE,
            thread_a::IPC_EP_BADGE,
            thread_a::IPC_BUFFER_VADDR,
            Some(thread_b_ipc_ep_cap), // give thread A access to thread B's IPC ep
            thread_a::run,
        );

        self.start_threads();

        Some(global_fault_ep_cap)
    }

    pub fn is_fault(&self, badge: seL4_Word) -> bool {
        for ref thread in self.thread_infos.iter() {
            if thread.fault_ep_badge == badge {
                return true;
            }
        }

        false
    }

    pub fn handle_fault(&self, badge: seL4_Word) {
        debug_println!("!!! thread faulted - badge = 0x{:X} !!!\n", badge);
        unsafe { seL4_DebugDumpScheduler() };
        debug_println!("");
    }

    fn create_ep(&mut self) -> seL4_CPtr {
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

    fn start_threads(&mut self) {
        for ref thread in self.thread_infos.iter() {
            let err = unsafe { seL4_TCB_Resume(thread.tcb_cap) };
            assert!(err == 0, "Failed to resume TCB");
        }
    }

    /// Create thread, does not start the thread
    fn create_thread(
        &mut self,
        fault_ep_cap: seL4_CPtr,
        fault_ep_badge: seL4_Word,
        ipc_ep_badge: seL4_Word,
        ipc_buffer_vaddr: seL4_Word,
        run_fn_ipc_ep_cap: Option<seL4_CPtr>,
        run_fn: fn(seL4_CPtr),
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
        let badged_fault_ep_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();
        let ipc_ep_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();
        let badged_ipc_ep_cap = self.bi_mngr.get_next_free_cap_slot().unwrap();

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

        let err = self.bi_mngr.untyped_retype_root(
            untyped_cap,
            api_object_seL4_EndpointObject,
            seL4_EndpointBits as _,
            ipc_ep_cap,
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
                badged_fault_ep_cap,
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
            seL4_CNode_Mint(
                cspace_cap,
                badged_ipc_ep_cap,
                seL4_WordBits as _,
                cspace_cap,
                ipc_ep_cap,
                seL4_WordBits as _,
                seL4_CapRights_new(1, 1, 1),
                ipc_ep_badge,
            )
        };
        assert!(err == 0, "Failed to mint a copy of the IPC endpoint");

        let err: seL4_Error = unsafe {
            seL4_TCB_Configure(
                tcb_cap,
                badged_fault_ep_cap,
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

        // create the thread's stack from the global allocator, which is backed by
        // a static array, just leak from box since it won't be given back
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

            // badged IPC ep cap in r0 is the function parameter
            if let Some(ipc_ep_arg) = run_fn_ipc_ep_cap {
                regs.r0 = ipc_ep_arg as _;
            } else {
                regs.r0 = badged_ipc_ep_cap as _;
            }
        }

        regs.sp = stack_top as seL4_Word;

        // using pc, sp, (cpsr) and r0
        let context_size = 4;
        let err = unsafe { seL4_TCB_WriteRegisters(tcb_cap, 0, 0, context_size, &mut regs) };
        assert!(err == 0, "Failed to write TCB registers");

        let err = unsafe { seL4_TCB_SetPriority(tcb_cap, seL4_CapInitThreadTCB.into(), 255) };
        assert!(err == 0, "Failed to set TCB priority");

        self.thread_infos.push(ThreadInfo {
            tcb_cap,
            fault_ep_cap: badged_fault_ep_cap,
            fault_ep_badge,
            ipc_ep_cap: badged_ipc_ep_cap,
            ipc_ep_badge,
        });
    }
}
