# percpu

[![Crates.io](https://img.shields.io/crates/v/percpu)](https://crates.io/crates/percpu)
[![Docs.rs](https://docs.rs/percpu/badge.svg)](https://docs.rs/percpu)
[![CI](https://github.com/arceos-org/percpu/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/arceos-org/percpu/actions/workflows/ci.yml)

Define and access per-CPU data structures.

All per-CPU data is placed into several contiguous memory regions called
**per-CPU data areas**, the number of which is the number of CPUs. Each CPU
has its own per-CPU data area. The architecture-specific per-CPU register
(e.g., `GS_BASE` on x86_64) is set to the base address of the area on
initialization.

When accessing the per-CPU data on the current CPU, it first use the per-CPU
register to obtain the corresponding per-CPU data area, and then add an offset
to access the corresponding field.

## Supported Architectures

| Architecture | per-CPU Register Used  |
| ---          | ---                    |
| ARM (32-bit) | `TPIDRURO` (c13)       |
| RISC-V       | `gp`                   |
| AArch64      | `TPIDR_ELx`            |
| x86_64       | `GS_BASE`              |
| LoongArch    | `$r21`                 |

> Notes for ARM (32-bit):
> We use `TPIDRURO` (User Read-Only Thread ID Register, CP15 c13) to store the
> per-CPU data area base address. This register is accessed via coprocessor
> instructions `mrc p15, 0, <Rt>, c13, c0, 3` (read) and
> `mcr p15, 0, <Rt>, c13, c0, 3` (write).

> Notes for RISC-V:
> Since RISC-V does not provide separate thread pointer registers for user and
> kernel mode, we temporarily use the `gp` register to point to the per-CPU data
> area, while the `tp` register is used for thread-local storage.

> Notes for AArch64:
> When feature `arm-el2` is enabled, `TPIDR_EL2` is used. Otherwise, `TPIDR_EL1`
> is used.

## Examples

```rust,no_run
#[percpu::def_percpu]
static CPU_ID: usize = 0;

// initialize per-CPU data areas.
#[cfg(not(feature = "custom-base"))]
percpu::init();
#[cfg(feature = "custom-base")]
{
    let cpu_count = 4;
    let size = percpu::percpu_area_size_for_cpus(cpu_count);
    let layout = std::alloc::Layout::from_size_align(size, 0x1000).unwrap();
    let base = unsafe { std::alloc::alloc(layout) as usize };
    percpu::init(base as *const (), cpu_count);
}
// set the thread pointer register to the per-CPU data area 0.
percpu::init_percpu_reg(0);

// access the per-CPU data `CPU_ID` on the current CPU.
println!("{}", CPU_ID.read_current()); // prints "0"
CPU_ID.write_current(1);
println!("{}", CPU_ID.read_current()); // prints "1"
```

Currently, you need to **modify the linker script manually**, add the following lines to your linker script:

```text,ignore
. = ALIGN(4K);
_percpu_start = .;
_percpu_end = _percpu_start + SIZEOF(.percpu);
.percpu 0x0 (NOLOAD) : AT(_percpu_start) {
    _percpu_load_start = .;
    *(.percpu .percpu.*)
    _percpu_load_end = .;
    . = _percpu_load_start + ALIGN(64) * CPU_NUM;
}
. = _percpu_end;
```

## Cargo Features

### Feature Modes

The crate supports different working modes through feature combinations:

| Mode | Features | `init()` Signature | Memory | VMA | Bare Metal | Linux |
|------|----------|-------------------|--------|-----|------------|-------|
| Default | (none) | `init() -> usize` | Linker `.percpu` | Must be 0 | ✅ | ❌ |
| sp-naive | `sp-naive` | `init(base?, count?)` | Global vars | N/A | ✅ | ✅ |
| non-zero-vma | `non-zero-vma` | `init() -> usize` | Linker `.percpu` | Any | ✅ | ✅ |
| custom-base | `custom-base` | `init(base, count) -> usize` | User-provided | Must be 0 | ✅ | ❌ |
| custom-base+non-zero-vma | `custom-base,non-zero-vma` | `init(base, count) -> usize` | User-provided | Any | ✅ | ✅ |

### Feature Descriptions

- `sp-naive`: For **single-core** use. Each per-CPU data is just a global variable,
  architecture-specific thread pointer register is not used.
- `preempt`: For **preemptible** system use. Disables preemption when accessing
  per-CPU data to prevent corruption.
- `arm-el2`: For **ARM system** running at **EL2** use (e.g. hypervisors).
  Uses `TPIDR_EL2` instead of `TPIDR_EL1`.
- `non-zero-vma`: Allows the `.percpu` section to be placed at a **non-zero VMA**.
  Required for Linux user-space and some linkers that don't support VMA 0.
- `custom-base`: Allows **user-defined memory allocation** for per-CPU data areas.
  Useful for dynamic CPU count or custom memory requirements.

### sp-naive Feature Interactions

When `sp-naive` is enabled with other features:
- **`custom-base`**: The `init(base, count)` signature is available but parameters are ignored.
- **`non-zero-vma`**: The feature is accepted but has no effect (no `.percpu` section is used).

This allows for a consistent API regardless of feature combination.

### Environment Differences

| Aspect | Bare Metal (`target_os = "none"`) | Linux |
|--------|-----------------------------------|-------|
| Default mode (VMA 0) | ✅ Works | ❌ Linker rejects VMA 0 |
| `non-zero-vma` | Optional, slight overhead | Required for multi-CPU modes |
| `custom-base` alone | ✅ User memory at VMA 0 | ❌ Symbols still need non-zero VMA |
| Per-CPU register | Direct MSR/assembly | `arch_prctl` syscall (x86_64) |

### Why Each Mode Exists

1. **Default mode**: Optimal for bare metal kernels with static CPU count.
   Zero-offset addressing provides best performance.

2. **`non-zero-vma` mode**: Required for Linux user-space testing and
   linkers that don't support VMA 0. Slight performance overhead.

3. **`custom-base` mode**: For dynamic CPU count, custom memory allocators,
   or hot-pluggable CPUs. Uses VMA 0 for optimal performance.

4. **`sp-naive` mode**: Zero overhead for single-core systems. Per-CPU data
   becomes regular global variables.
