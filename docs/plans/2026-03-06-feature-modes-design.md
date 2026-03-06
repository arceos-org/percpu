# percpu Feature Modes Design

## Overview

This document describes the feature combinations (working modes) of the `percpu` crate, their behavior, testing strategy, and documentation updates.

## Feature Modes

### Core Modes

| Mode | Features | `init()` signature | Memory | VMA | Bare Metal | Linux |
|------|----------|-------------------|--------|-----|------------|-------|
| **Default** | (none) | `init() -> usize` | Linker `.percpu` | Must be 0 | ✅ | ❌ |
| **sp-naive** | `sp-naive` [+ others] | `init(base?, count?)` | Global vars | N/A | ✅ | ✅ |
| **non-zero-vma** | `non-zero-vma` | `init() -> usize` | Linker `.percpu` | Any | ✅ | ✅ |
| **custom-base** | `custom-base` | `init(base, count) -> usize` | User-provided | Must be 0 | ✅ | ❌ |
| **custom-base+non-zero-vma** | `custom-base,non-zero-vma` | `init(base, count) -> usize` | User-provided | Any | ✅ | ✅ |

### sp-naive Feature Interactions

When `sp-naive` is enabled, other features are accepted but have no effect:

- **`custom-base`**: `init(base, count)` signature is available, but parameters are ignored
- **`non-zero-vma`**: Accepted but has no effect (no `.percpu` section is used)

This allows for a consistent API regardless of feature combination.

### Orthogonal Features

These features work with all core modes:

- **`preempt`**: Adds preemption guards during per-CPU data access. Works with all modes.
- **`arm-el2`**: Uses `TPIDR_EL2` instead of `TPIDR_EL1` on AArch64. Only meaningful on ARM architecture.

## Environment Differences

### Bare Metal vs Linux

| Aspect | Bare Metal (`target_os = "none"`) | Linux |
|--------|-----------------------------------|-------|
| Default mode (VMA 0) | ✅ Works | ❌ Linker rejects VMA 0 |
| `non-zero-vma` | Optional, slight overhead | Required for multi-CPU modes |
| `custom-base` alone | ✅ User memory at VMA 0 | ❌ Symbols still need non-zero VMA |
| Per-CPU register | Direct MSR/assembly | `arch_prctl` syscall (x86_64) |
| Memory allocation in `init()` | Copies `.percpu` section to each CPU area | Auto-allocates if not `custom-base` |

### Why Each Mode Exists

1. **Default mode**: Designed for bare metal kernels where:
   - The linker script defines `.percpu` at VMA 0
   - Zero-offset addressing provides optimal performance
   - The kernel controls per-CPU register initialization

2. **`non-zero-vma` mode**: Required when:
   - Running on Linux user-space (linker doesn't allow VMA 0)
   - Using certain linkers that don't support zero VMA
   - Slightly slower due to VMA offset calculations

3. **`custom-base` mode**: Required when:
   - Dynamic CPU count (not known at link time)
   - Custom memory allocator needed
   - Hot-pluggable CPUs
   - Memory must be at specific address
   - `.percpu` section still at VMA 0 for optimal performance

4. **`sp-naive` mode**: For single-core systems where:
   - No per-CPU register manipulation needed
   - Zero overhead vs regular global variable
   - Can still use `preempt` if OS has preemption

## Test Matrix

### Linux Testable Configurations (x86_64)

| # | Features | Description |
|---|----------|-------------|
| 1 | `sp-naive` | Single-CPU mode |
| 2 | `sp-naive,custom-base` | Single-CPU with custom-base API (ignored) |
| 3 | `non-zero-vma` | Multi-CPU with linker-defined area |
| 4 | `custom-base,non-zero-vma` | Multi-CPU with user-provided memory |

**Note**: `non-zero-vma` is required for all multi-CPU modes on Linux.

### Bare Metal Build Checks

All targets should be checked with:
- `(no features)` - Default mode
- `custom-base` - Custom memory with VMA 0
- `custom-base,non-zero-vma` - Custom memory with non-zero VMA
- `non-zero-vma` - Non-zero VMA
- `preempt,arm-el2` - With orthogonal features
- `preempt,arm-el2,non-zero-vma` - With orthogonal features

## Implementation Plan

### Code Changes

1. **percpu/src/naive.rs**: Update `init()` signature when `custom-base` is enabled to accept (but ignore) base and count parameters.

2. **percpu/tests/test_percpu.rs**:
   - Refactor test code with shared helper functions
   - Update `#[cfg]` attribute to allow tests with `sp-naive` without `non-zero-vma`
   - Consolidate initialization logic

3. **.github/workflows/test.yml**: Add test configurations:
   - `sp-naive`
   - `sp-naive,custom-base`
   - `non-zero-vma`
   - `custom-base,non-zero-vma`

4. **.github/workflows/check.yml**: Add build checks:
   - `custom-base`
   - `custom-base,non-zero-vma`

5. **README.md**: Update documentation:
   - Feature modes table
   - Environment differences section
   - Why each mode exists
   - sp-naive interactions note

## Code Changes Detail

### naive.rs init() signature update

```rust
/// Initialize all per-CPU data areas.
///
/// For "sp-naive" use it does nothing and returns `1`.
/// When "custom-base" feature is enabled, the parameters are accepted but ignored.
#[cfg(not(feature = "custom-base"))]
pub fn init() -> usize {
    1
}

#[cfg(feature = "custom-base")]
pub fn init(_base: *const (), _cpu_count: usize) -> usize {
    1
}
```

### Test refactoring

The test should use a helper function for initialization that handles all modes:

```rust
fn setup_percpu() -> usize {
    #[cfg(feature = "sp-naive")]
    {
        #[cfg(feature = "custom-base")]
        init(std::ptr::null(), 1);
        #[cfg(not(feature = "custom-base"))]
        init();
        0
    }

    #[cfg(all(not(feature = "sp-naive"), not(feature = "custom-base")))]
    {
        assert_eq!(init(), 4);
        let base = percpu_area_base(0);
        unsafe { write_percpu_reg(base) };
        read_percpu_reg()
    }

    #[cfg(all(not(feature = "sp-naive"), feature = "custom-base"))]
    {
        let size = percpu_area_size_for_cpus(4);
        let layout = std::alloc::Layout::from_size_align(size, 0x1000).unwrap();
        let base = unsafe { std::alloc::alloc(layout) as usize };
        assert_eq!(init(base as *const (), 4), 4);
        init_percpu_reg(0);
        base
    }
}
```