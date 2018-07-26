# fel4-test-project
A feL4 test project

Rust running on seL4 threads.

## Building

```bash
./scripts/prepare-repos

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

thread_a::run()
thread_a::ep_cap = 0x2119 - aux_ep_cap = 0x2114
thread_a::sending message to B
thread_b::run()
thread_b::ep_cap = 0x2114 - aux_ep_cap = 0x0
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_a::sending message to B
thread_b::got msg from A, sending reply
thread_b::done
!!! thread faulted - badge = 0xB !!!

Dumping all tcbs!
Name                                            State           IP                       Prio    Core
--------------------------------------------------------------------------------------
child of: 'rootserver'                          running         0x22fe4 255                     0
child of: 'rootserver'                          blocked on reply        (nil)   255                   0
idle_thread                                     idle            (nil)   0                       0
rootserver                                      running         0x22fe4 255                     0

thread_a::done
!!! thread faulted - badge = 0xA !!!

Dumping all tcbs!
Name                                            State           IP                       Prio    Core
--------------------------------------------------------------------------------------
child of: 'rootserver'                          blocked on reply        (nil)   255                   0
child of: 'rootserver'                          blocked on reply        (nil)   255                   0
idle_thread                                     idle            (nil)   0                       0
rootserver                                      running         0x22fe4 255                     0
```
