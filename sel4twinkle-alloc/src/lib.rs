#![no_std]

extern crate sel4_sys;

use sel4_sys::seL4_CPtr;

pub const MIN_UNTYPED_SIZE: usize = 4;
pub const MAX_UNTYPED_SIZE: usize = 32;

pub const MAX_UNTYPED_ITEMS: usize = 256;

struct untyped_item {
    cap: seL4_CPtr,
}
