use super::Allocator;
use sel4_sys::{
    api_object_seL4_CapTableObject, api_object_seL4_EndpointObject,
    api_object_seL4_NotificationObject, api_object_seL4_TCBObject, api_object_seL4_UntypedObject,
    seL4_EndpointBits, seL4_NotificationBits, seL4_SlotBits, seL4_TCBBits, seL4_Word,
};

impl Allocator {
    /// Get the size (in bits) of the untyped memory required to create an
    /// object of the given size.
    ///
    /// TODO - see vka/object.h, not handling all cases yet (feature gating for
    /// RT/etc)
    pub fn vka_get_object_size(&self, obj_type: seL4_Word, obj_size_bits: usize) -> usize {
        match obj_type {
            api_object_seL4_UntypedObject => obj_size_bits as _,
            api_object_seL4_TCBObject => seL4_TCBBits as _,
            api_object_seL4_EndpointObject => seL4_EndpointBits as _,
            api_object_seL4_NotificationObject => seL4_NotificationBits as _,
            api_object_seL4_CapTableObject => (seL4_SlotBits as usize + obj_size_bits),
            //seL4_KernelImageObject => seL4_KernelImageBits,
            _ => panic!("vka_arch_get_object_size() not implemented"),
        }
    }
}
