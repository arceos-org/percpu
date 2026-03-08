# Simplify Mode Design

## Summary

Simplify the percpu crate's mode system:
1. Remove `custom-base` feature
2. Keep only two modes: single-core (`sp-naive`) and multi-core (default)
3. Provide both `init(base, cpu_count)` and `init_static()` in both modes
4. Always use global variable `PERCPU_AREA_BASE` in multi-core mode

## Feature Definitions

### Two Modes (via `sp-naive` feature)

| Feature | Mode | Description |
|---------|------|-------------|
| `sp-naive` | Single-core | Per-CPU data stored in global variables |
| (none) | Multi-core | Use `.percpu` section or user-provided memory |

### Removed Feature

- `custom-base` — No longer needed

### Retained Auxiliary Features

- `non-zero-vma`: Allow `.percpu` section at non-zero VMA
- `preempt`: Preemptible system support
- `arm-el2`: ARM EL2 privilege level support

## Init Function API

### `init_static()` — Static initialization

```rust
/// Initialize per-CPU data areas using the `.percpu` section.
///
/// - Multi-core mode: copies primary CPU's data to other CPUs, returns CPU count
/// - Single-core mode: no initialization needed, returns 1
pub fn init_static() -> usize;
```

### `init(base, cpu_count)` — Dynamic initialization

```rust
/// Initialize per-CPU data areas with user-provided memory.
///
/// - Multi-core mode: uses the provided base address, stores it in PERCPU_AREA_BASE
/// - Single-core mode: ignores parameters, returns 1
///
/// # Arguments
/// - `base`: Base address of user-allocated memory
/// - `cpu_count`: Number of CPUs
pub fn init(base: *const (), cpu_count: usize) -> usize;
```

### Helper Function (multi-core mode only)

```rust
/// Calculate memory size for given CPU count.
pub fn percpu_area_size_for_cpus(cpu_count: usize) -> usize;
```

## Implementation

### naive.rs (Single-core Mode)

```rust
pub fn init_static() -> usize { 1 }

pub fn init(_base: *const (), _cpu_count: usize) -> usize { 1 }
```

### imp.rs (Multi-core Mode)

```rust
// Global variable: stores user-provided base address
static PERCPU_AREA_BASE: spin::once::Once<usize> = spin::once::Once::new();

// Modified percpu_area_base: prefer PERCPU_AREA_BASE
pub fn percpu_area_base(cpu_id: usize) -> usize {
    let base = PERCPU_AREA_BASE.get()
        .map(|b| *b)
        .unwrap_or(_percpu_start as *const () as usize);
    base + cpu_id * align_up_64(percpu_area_size())
}

// init_static: calls init with _percpu_start
pub fn init_static() -> usize {
    init(_percpu_start as *const (), percpu_area_num())
}

// init: user-provided base address
pub fn init(base: *const (), cpu_count: usize) -> usize {
    PERCPU_AREA_BASE.call_once(|| base as _);
    copy_percpu_region(_percpu_start as *const (), 0..cpu_count);
    cpu_count
}

pub fn percpu_area_size_for_cpus(cpu_count: usize) -> usize {
    cpu_count * align_up_64(percpu_area_size())
}
```

## File Changes

| File | Changes |
|------|---------|
| `percpu/Cargo.toml` | Remove `custom-base` feature |
| `percpu_macros/Cargo.toml` | Remove `custom-base` feature |
| `percpu/src/naive.rs` | Add `init()` function |
| `percpu/src/imp.rs` | Remove `#[cfg(feature = "custom-base")]`, add `PERCPU_AREA_BASE`, modify `init_static()` to call `init()` |
| `percpu/tests/test_percpu.rs` | Remove `custom-base` tests, update test logic |
| `README.md` | Update mode documentation and examples |
| `Cargo.toml` (workspace) | Version update |

## Migration Guide

### For sp-naive users

```rust
// Before (0.3.x)
percpu::init_static();

// After (0.4.0) - no change needed
percpu::init_static();

// Or use init() (parameters ignored)
percpu::init(std::ptr::null(), 1);
```

### For multi-core static users

```rust
// Before (0.3.x)
percpu::init_static();

// After (0.4.0) - no change needed
percpu::init_static();
```

### For custom-base users

```rust
// Before (0.3.x)
percpu::init_dynamic(base, cpu_count);

// After (0.4.0) - use init()
percpu::init(base, cpu_count);
```