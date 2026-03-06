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
    // when `custom-base` feature is enabled, you need to allocate the per-CPU
    // data area manually.
    let cpu_count = 4;
    let size = percpu::percpu_area_size_for_cpus(cpu_count);
    let layout = std::alloc::Layout::from_size_align(size, 0x1000).unwrap();
    let base = unsafe { std::alloc::alloc(layout) as usize };
    percpu::init(base as *const (), cpu_count);
    // and set the initial value manually.
    CPU_ID.reset_to_init();
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

## Notes

### Working Modes

The crate supports different working modes through feature combinations:

| Features                         | Per-CPU Data Area | `.percpu` VMA | Use case                                      | Linux |
|----------------------------------|-------------------|---------------|-----------------------------------------------|-------|
| (none)                           | `.percpu` section | Must be 0     | Multi-threaded bare metal                     | ❌     |
| `sp-naive`                       | Global vars       | N/A           | Single-threaded bare metal / Linux user-space | ✅     |
| `non-zero-vma`                   | `.percpu` section | Any           | Multi-threaded Linux user-space               | ✅     |
| `custom-base`                    | Custom memory     | Must be 0     | PIC bare metal / Dynamic CPU detection        | ❌     |
| `custom-base` & `non-zero-vma` | Custom memory     | Any           | PIC Linux user-space                          | ✅     |

### Cargo Features

These features control the working mode of the crate:

- `sp-naive`: Force **single-core** mode. Each per-CPU data is just a global variable,
  architecture-specific thread pointer register is not used. This feature **disables** `non-zero-vma` and `custom-base`.
- `non-zero-vma`: Allows the `.percpu` section to be placed at a **non-zero VMA**.
  Required for Linux user-space programs as some linkers that don't support VMA 0.
- `custom-base`: Allows **user-defined memory allocation** for per-CPU data areas.
  Useful for dynamic CPU count or custom memory requirements.

These features further control the behavior of the crate:

- `preempt`: For **preemptible** system use. Disables preemption when accessing
  per-CPU data to prevent corruption.
- `arm-el2`: For **ARM system** running at **EL2** use (e.g. hypervisors).
  Uses `TPIDR_EL2` instead of `TPIDR_EL1`.

### Default values

The default values of per-CPU static variables **ARE NOT** assigned when:
- `custom-base` feature is enabled, or
- not running on bare metal.

In these cases, you need to set the initial value manually.

```rust,no_run
#[def_percpu]
static CPU_ID: usize = 42;

CPU_ID.reset_to_init();
```
