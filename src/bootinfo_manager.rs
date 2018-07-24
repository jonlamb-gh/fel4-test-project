use sel4_sys::*;

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

pub struct BootInfoManager {
    bootinfo: &'static seL4_BootInfo,
    empty_used: seL4_CPtr,
    cspace_cap: seL4_CPtr,
    pd_cap: seL4_CPtr,
    page_table_cap: seL4_CPtr,
}

impl BootInfoManager {
    pub fn new(bootinfo: &'static seL4_BootInfo) -> BootInfoManager {
        unsafe { seL4_SetUserData(bootinfo.ipcBuffer as _) };

        BootInfoManager {
            bootinfo,
            empty_used: 1 as _, // page_table_cap reserves the first empty slot
            cspace_cap: seL4_CapInitThreadCNode,
            pd_cap: seL4_CapInitThreadVSpace,
            page_table_cap: bootinfo.empty.start,
        }
    }

    pub fn io_map(
        &mut self,
        untyped_cap: seL4_CPtr,
        paddr: seL4_Word,
        vaddr: seL4_Word,
        size_bits: usize,
    ) -> seL4_Error {
        debug_println!(
            "io_map: mapping frame paddr 0x{:X} -> vaddr 0x{:X} - size = {}",
            paddr,
            vaddr,
            (1 << size_bits)
        );

        if let Some(frame_cap) = self.get_frame_cap(paddr, size_bits) {
            self.map_paddr(untyped_cap, frame_cap, vaddr)
        } else {
            panic!("Failed to get frame cap");
        }
    }

    pub fn map_paddr(
        &self,
        untyped_cap: seL4_CPtr,
        frame_cap: seL4_CPtr,
        vaddr: seL4_Word,
    ) -> seL4_Error {
        // memory mapped IO device region, no cache attributes
        let cache_attribs: seL4_ARM_VMAttributes = 0;

        // cap rights grant, read, write
        let cap_grant = 0;
        let cap_read = 1;
        let cap_write = 1;

        let map_err: seL4_Error = unsafe {
            seL4_ARM_Page_Map(
                frame_cap,
                self.pd_cap,
                vaddr,
                seL4_CapRights_new(cap_grant, cap_read, cap_write),
                cache_attribs,
            )
        };

        // TODO - make this better
        if map_err != 0 {
            let err = self.untyped_retype_root(
                untyped_cap,
                _object_seL4_ARM_PageTableObject,
                seL4_PageTableBits as usize,
                self.page_table_cap,
            );

            if err != 0 {
                return err;
            }

            let err: seL4_Error = unsafe {
                seL4_ARM_PageTable_Map(self.page_table_cap, self.pd_cap, vaddr, cache_attribs)
            };

            if err != 0 {
                return err;
            }

            let err: seL4_Error = unsafe {
                seL4_ARM_Page_Map(
                    frame_cap,
                    self.pd_cap,
                    vaddr,
                    seL4_CapRights_new(cap_grant, cap_read, cap_write),
                    cache_attribs,
                )
            };

            if err != 0 {
                return err;
            }
        }
        0
    }

    pub fn get_frame_cap(&mut self, paddr: seL4_Word, size_bits: usize) -> Option<seL4_CPtr> {
        if let Some(dest_slot_cap) = self.get_next_free_cap_slot() {
            if let Some(untyped_cap) = self.get_untyped(Some(paddr), 1 << size_bits) {
                let err = self.untyped_retype_root(
                    untyped_cap,
                    _object_seL4_ARM_SmallPageObject,
                    size_bits,
                    dest_slot_cap,
                );

                if err == 0 {
                    return Some(dest_slot_cap);
                }
            }
        }

        None
    }

    pub fn get_untyped(&self, paddr: Option<seL4_Word>, size_bytes: usize) -> Option<seL4_CPtr> {
        for i in self.bootinfo.untyped.start..self.bootinfo.untyped.end {
            let idx: usize = (i - self.bootinfo.untyped.start) as usize;
            if (1 << self.bootinfo.untypedList[idx].sizeBits) as usize >= size_bytes {
                if let Some(paddr) = paddr {
                    if self.bootinfo.untypedList[idx].paddr == paddr {
                        return Some(i);
                    }
                } else {
                    return Some(i);
                }
            }
        }
        None
    }

    /// TODO - error handling
    pub fn get_next_free_cap_slot(&mut self) -> Option<seL4_CPtr> {
        let offset = self.empty_used;
        self.empty_used += 1;
        Some(self.bootinfo.empty.start + offset)
    }

    /// TODO - maybe use a cspacepath_t object here?
    /// Retypes an untyped object to the specified object of specified size,
    /// storing a cap to that object in the specified slot of the cspace
    /// whose root is root_cnode. This requires that the root_cnode
    /// argument is also the root cnode of the cspace of the calling thread.
    pub fn untyped_retype_root(
        &self,
        untyped_cap: seL4_CPtr,
        obj_type: seL4_ObjectType,
        size_bits: usize,
        slot_cap: seL4_CPtr,
    ) -> seL4_Error {
        unsafe {
            seL4_Untyped_Retype(
                untyped_cap,
                obj_type,
                size_bits as seL4_Word,
                self.cspace_cap,
                self.cspace_cap,
                seL4_WordBits.into(),
                slot_cap,
                1,
            )
        }
    }

    pub fn debug_print_bootinfo(&self) {
        unsafe {
            debug_println!("------------- bootinfo -------------");
            debug_println!("bootinfo.empty.start = {}", self.bootinfo.empty.start);
            debug_println!("bootinfo.empty.end = {}", self.bootinfo.empty.end);

            debug_println!(
                "bootinfo.userImageFrames.start = {}",
                self.bootinfo.userImageFrames.start
            );
            debug_println!(
                "bootinfo.userImageFrames.end = {}",
                self.bootinfo.userImageFrames.end
            );

            debug_println!("bootinfo.untyped.start = {}", self.bootinfo.untyped.start);
            debug_println!("bootinfo.untyped.end = {}", self.bootinfo.untyped.end);

            debug_println!("bootinfo.untypedList");
            debug_println!(
                "  length = {}",
                self.bootinfo.untyped.end - self.bootinfo.untyped.start
            );

            for i in self.bootinfo.untyped.start..self.bootinfo.untyped.end {
                let index: usize = (i - self.bootinfo.untyped.start) as usize;
                debug_println!(
                    "  [{} | {}] paddr = 0x{:X} - size_bits = {} - is_device = {}",
                    index,
                    i,
                    self.bootinfo.untypedList[index].paddr,
                    self.bootinfo.untypedList[index].sizeBits,
                    self.bootinfo.untypedList[index].isDevice
                );
            }
            debug_println!("--------------------------\n");
        }
    }
}
