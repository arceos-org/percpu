# Mode Refactoring Design

## Summary

Refactor the percpu crate's mode system to provide clearer API boundaries:
1. Define 3 distinct modes: single-core, multi-core static, multi-core dynamic
2. Decouple `non-zero-vma` from modes (becomes an independent feature)
3. Split `init()` into `init_static()` and `init_dynamic()` for better API clarity

## Feature Definitions

### Three Modes (mutually exclusive, priority-based selection)

| Feature | Mode | Memory Source | Use Case |
|---------|------|---------------|----------|
| `sp-naive` | Single-core | Global variables | Single-threaded bare metal / Linux |
| (none) | Multi-core static | `.percpu` section | Multi-threaded bare metal |
| `custom-base` | Multi-core dynamic | User-allocated | Dynamic CPU count / PIC |

### Auxiliary Features (combinable with any mode)

| Feature | Purpose |
|---------|---------|
| `non-zero-vma` | Allow `.percpu` section at non-zero VMA |
| `preempt` | Preemptible system support |
| `arm-el2` | ARM EL2 privilege level support |

## Init Function API

### `init_static()` — For single-core and multi-core static modes

```rust
/// Initialize per-CPU data areas.
///
/// Available in `sp-naive` and default (multi-core static) modes.
///
/// - `sp-naive` mode: No initialization needed, returns 1
/// - Default mode: Initialize using `.percpu` section, returns CPU count
///
/// Returns the number of areas initialized. Returns 0 if already called.
#[cfg(not(feature = "custom-base"))]
pub fn init_static() -> usize;
```

### `init_dynamic(base, cpu_count)` — For multi-core dynamic mode

```rust
/// Initialize per-CPU data areas with user-provided memory.
///
/// Only available in `custom-base` mode.
///
/// # Arguments
/// - `base`: Base address of user-allocated memory
/// - `cpu_count`: Number of CPUs
///
/// Returns the number of areas initialized. Can be called repeatedly for re-initialization.
#[cfg(feature = "custom-base")]
pub fn init_dynamic(base: *const (), cpu_count: usize) -> usize;
```

### Helper Function (custom-base mode only)

```rust
/// Calculate memory size required for given CPU count.
#[cfg(feature = "custom-base")]
pub fn percpu_area_size_for_cpus(cpu_count: usize) -> usize;
```

## Implementation

### File Structure

```
percpu/src/
├── lib.rs           # Export public API
├── naive.rs         # sp-naive mode implementation
└── imp.rs           # Multi-core mode implementation (static + dynamic)
```

### Mode Selection Logic (in `lib.rs`)

```rust
#[cfg_attr(feature = "sp-naive", path = "naive.rs")]
mod imp;

// Export init functions based on mode
#[cfg(not(feature = "custom-base"))]
pub use self::imp::init_static;

#[cfg(feature = "custom-base")]
pub use self::imp::init_dynamic;
```

### Key Changes

#### `naive.rs` (sp-naive mode)

- Rename `init()` to `init_static()`
- Remove `custom-base` conditional code
- Remove `percpu_area_size_for_cpus()` (not needed in sp-naive)

#### `imp.rs` (multi-core modes)

- Rename `init()` to either `init_static()` or `init_dynamic()`
- Expose appropriate function based on `custom-base` feature
- Keep `non-zero-vma` logic unchanged

## Testing

### Test Updates

`test_percpu.rs` needs mode-specific init calls:

```rust
#[cfg(not(feature = "custom-base"))]
percpu::init_static();

#[cfg(feature = "custom-base")]
percpu::init_dynamic(base, cpu_count);
```

### CI Test Matrix

| Mode | non-zero-vma | Test Command |
|------|--------------|--------------|
| sp-naive | off | `--features sp-naive` |
| sp-naive | on | `--features "sp-naive,non-zero-vma"` |
| default | off | (no features) |
| default | on | `--features non-zero-vma` |
| custom-base | off | `--features custom-base` |
| custom-base | on | `--features "custom-base,non-zero-vma"` |

## Documentation Updates

1. **README.md**: Update Working Modes table and example code
2. **API docs**: Update function documentation comments
3. **Examples**: Replace `init()` with `init_static()` or `init_dynamic()`

## Version Update

- Version: `0.2.2` → `0.3.0` (breaking change)
- This is a breaking API change (init function signature)

## Migration Guide

### For sp-naive users

```rust
// Before (0.2.x)
percpu::init();

// After (0.3.0)
percpu::init_static();
```

### For default mode users

```rust
// Before (0.2.x)
percpu::init();

// After (0.3.0)
percpu::init_static();
```

### For custom-base users

```rust
// Before (0.2.x)
percpu::init(base, cpu_count);

// After (0.3.0)
percpu::init_dynamic(base, cpu_count);
```