#![no_std]
#![cfg_attr(feature = "alloc", feature(alloc))]

#[cfg(all(feature = "alloc"))]
#[macro_use]
extern crate alloc;

extern crate sel4_sys;

#[cfg(all(feature = "test"))]
#[macro_use]
extern crate proptest;

#[cfg(feature = "test")]
pub mod fel4_test;

#[macro_use]
mod macros;

use sel4_sys::{seL4_BootInfo, seL4_CPtr};

#[cfg(feature = "KernelPrinting")]
use sel4_sys::DebugOutHandle;

/// Returns cap to global fault endpoint if so desired (TODO and handler fn?)
pub fn init(bootinfo: &'static seL4_BootInfo) -> Option<seL4_CPtr> {
    debug_println!("lib::init()");

    None
}

pub fn run() {
    debug_println!("lib::run()");
}
