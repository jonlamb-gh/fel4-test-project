# fel4-test-project
A feL4 test project

Rust running on seL4 threads.

## Building

```bash
cargo fel4 build

cargo fel4 simulate
```

## Output

```bash
ELF-loader started on CPU: ARM Ltd. Cortex-A9 r0p0
  paddr=[20000000..20113fff]
ELF-loading image 'kernel'
  paddr=[10000000..10032fff]
  vaddr=[e0000000..e0032fff]
  virt_entry=e0000000
ELF-loading image 'root-task'
  paddr=[10033000..10065fff]
  vaddr=[10000..42fff]
  virt_entry=100f8
Enabling MMU and paging
Jumping to kernel-image entry point...

Bootstrapping kernel
Booting all finished, dropped to user space
------------- bootinfo -------------
bootinfo.empty.start = 225
bootinfo.empty.end = 524288
bootinfo.userImageFrames.start = 13
bootinfo.userImageFrames.end = 64
bootinfo.untyped.start = 64
bootinfo.untyped.end = 225
bootinfo.untypedList
  length = 161
  [0 | 64] paddr = 0x10000000 - size_bits = 16 - is_device = 0
  [1 | 65] paddr = 0x10066000 - size_bits = 13 - is_device = 0

... lots of debug output ...

--------------------------

lib::run()
lib::run() about to fault

root-task thread got notification badge 0x61
!!! thread faulted !!!
```