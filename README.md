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

// Initialize per-CPU data areas.
#[cfg(not(feature = "custom-base"))]
percpu::init_static();
#[cfg(feature = "custom-base")]
{
    // When `custom-base` feature is enabled, you need to allocate the per-CPU
    // data area manually.
    let cpu_count = 4;
    let size = percpu::percpu_area_size_for_cpus(cpu_count);
    let layout = std::alloc::Layout::from_size_align(size, 0x1000).unwrap();
    let base = unsafe { std::alloc::alloc(layout) as usize };
    percpu::init_dynamic(base as *const (), cpu_count);
    // And set the initial value manually.
    CPU_ID.reset_to_init();
}
// Set the thread pointer register to the per-CPU data area 0.
percpu::init_percpu_reg(0);

// Access the per-CPU data `CPU_ID` on the current CPU.
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

## Notes

### Working Modes

The crate supports three working modes through feature combinations:

| Mode | Feature | Per-CPU Data Area | `.percpu` VMA | Init Function | Use case |
|------|---------|-------------------|---------------|---------------|----------|
| Single-core | `sp-naive` | Global vars | N/A | `init_static()` | Single-threaded bare metal / Linux |
| Multi-core static | (none) | `.percpu` section | Must be 0 | `init_static()` | Multi-threaded bare metal |
| Multi-core dynamic | `custom-base` | Custom memory | Must be 0 | `init_dynamic()` | PIC bare metal / Dynamic CPU count |

The `non-zero-vma` feature can be combined with any mode to allow the `.percpu` section to be placed at a non-zero VMA address.

### Cargo Features

**Mode features** (select one):

- `sp-naive`: **Single-core** mode. Each per-CPU data is just a global variable. Architecture-specific thread pointer register is not used. Use `init_static()` for initialization.
- `custom-base`: **Multi-core dynamic** mode. Allows user-defined memory allocation for per-CPU data areas. Use `init_dynamic()` for initialization.
- (none): **Multi-core static** mode (default). Uses `.percpu` section for per-CPU data areas. Use `init_static()` for initialization.

**Auxiliary features** (can be combined with any mode):

- `non-zero-vma`: Allows the `.percpu` section to be placed at a **non-zero VMA**. Required for Linux user-space programs as some linkers don't support VMA 0.
- `preempt`: For **preemptible** system use. Disables preemption when accessing per-CPU data to prevent corruption.
- `arm-el2`: For **ARM system** running at **EL2** use (e.g. hypervisors). Uses `TPIDR_EL2` instead of `TPIDR_EL1`.

### Default values

The default values of per-CPU static variables **ARE NOT** assigned when:
- `custom-base` feature is enabled, or
- not running on bare metal.

In these cases, you need to set the initial value manually.

```rust,no_run
use percpu::def_percpu;

#[def_percpu]
static CPU_ID: usize = 42;

CPU_ID.reset_to_init();
```
