# percpu

[![Crates.io](https://img.shields.io/crates/v/percpu)](https://crates.io/crates/percpu)
[![Docs.rs](https://docs.rs/percpu/badge.svg)](https://docs.rs/percpu)
[![CI](https://github.com/arceos-org/percpu/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/arceos-org/arceos/actions/workflows/ci.yml)

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

// Option 1: Use init_static() for static initialization (uses .percpu section)
percpu::init_static();
percpu::init_percpu_reg(0);

// Option 2: Use init() for dynamic initialization (user-provided memory)
// let cpu_count = 4;
// let size = percpu::percpu_area_size_for_cpus(cpu_count);
// let layout = std::alloc::Layout::from_size_align(size, 0x1000).unwrap();
// let base = unsafe { std::alloc::alloc(layout) as usize };
// percpu::init(base as *const (), cpu_count);
// percpu::init_percpu_reg(0);

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

The crate supports two working modes:

| Mode        | Feature    | Per-CPU Data Area | `.percpu` VMA | Use case                           |
|-------------|------------|-------------------|---------------|------------------------------------|
| Single-core | `sp-naive` | Global vars       | N/A           | Single-threaded bare metal / Linux |
| Multi-core  | (none)     | `.percpu` section | Must be 0     | Multi-threaded bare metal          |

### Initialization Functions

Both modes provide the same initialization API:

- `init_static()`: Initialize using the `.percpu` section (static allocation)
  - Multi-core: Uses `_percpu_start` as base address
  - Single-core: No-op, returns 1

- `init(base, cpu_count)`: Initialize with user-provided memory (dynamic allocation)
  - Multi-core: Uses the provided base address
  - Single-core: Parameters ignored, returns 1

- `percpu_area_size_for_cpus(cpu_count)`: Calculate memory size for given CPU count
  - Multi-core: Returns `cpu_count * percpu_area_size()`
  - Single-core: Not available (returns 0)

### Features List

**Mode feature**:

- `sp-naive`: **Single-core** mode. Each per-CPU data is just a global variable.
  Architecture-specific thread pointer register is not used.

**Auxiliary features** (can be combined with any mode):

- `non-zero-vma`: Allows the `.percpu` section to be placed at a **non-zero VMA**.
  Required for Linux user-space programs as some linkers don't support VMA 0.
  `sp-naive` does not need `non-zero-vma` as it uses global variables.
- `preempt`: For **preemptible** system use. Disables preemption when accessing
  per-CPU data to prevent corruption.
- `arm-el2`: For **ARM system** running at **EL2** use (e.g. hypervisors).
  Uses `TPIDR_EL2` instead of `TPIDR_EL1`.