# Implementation Plan: Simplify Mode

## Overview

This plan implements the simplify mode design to remove `custom-base` feature and unify the init API.

## Prerequisites

- [x] Design document approved: `docs/plans/2026-03-08-simplify-mode-design.md`

## Implementation Steps

### Step 1: Update Cargo.toml files

**Files**: `Cargo.toml`, `percpu/Cargo.toml`, `percpu_macros/Cargo.toml`

**Changes**:
1. Remove `custom-base` feature definition from `percpu/Cargo.toml`
2. Remove `custom-base` feature definition from `percpu_macros/Cargo.toml`
3. Update version from `0.3.0` to `0.4.0`

**Verification**:
```bash
cargo check --workspace
```

---

### Step 2: Update naive.rs (single-core mode)

**File**: `percpu/src/naive.rs`

**Changes**:
1. Add `init(base, cpu_count)` function that ignores parameters and returns 1

**Verification**:
```bash
cargo check -p percpu --features sp-naive
```

---

### Step 3: Update imp.rs (multi-core mode)

**File**: `percpu/src/imp.rs`

**Changes**:
1. Move `PERCPU_AREA_BASE` outside of `#[cfg(feature = "custom-base")]` - always define it
2. Modify `percpu_area_base()` to use `PERCPU_AREA_BASE.get()` first, fallback to `_percpu_start`
3. Rename `init_dynamic` to `init` (remove `#[cfg(feature = "custom-base")]`)
4. Modify `init_static()` to call `init(_percpu_start as *const (), percpu_area_num())`
5. Remove `#[cfg(feature = "custom-base")]` from `percpu_area_size_for_cpus()`
6. Remove the `IS_INIT` re-initialization logic (simplify)

**Verification**:
```bash
cargo check -p percpu
cargo check -p percpu --features non-zero-vma
```

---

### Step 4: Update lib.rs exports

**File**: `percpu/src/lib.rs`

**Changes**:
- No changes needed (functions are exported via `pub use self::imp::*`)

**Verification**:
```bash
cargo check -p percpu
```

---

### Step 5: Update tests

**File**: `percpu/tests/test_percpu.rs`

**Changes**:
1. Remove `test_percpu_custom_base` test function
2. Modify test conditions to remove `custom-base` feature checks
3. Add test for `init()` function in default mode
4. Update test for `init()` function in sp-naive mode

**Verification**:
```bash
cargo test -p percpu --features sp-naive
cargo test -p percpu --features non-zero-vma
```

---

### Step 6: Update README.md

**File**: `percpu/README.md`

**Changes**:
1. Remove `custom-base` from mode table
2. Update examples to show both `init()` and `init_static()`
3. Update feature descriptions
4. Update migration guide

**Verification**:
```bash
cargo test -p percpu --doc --features sp-naive
```

---

### Step 7: Run full test suite

**Commands**:
```bash
# Test all mode combinations
cargo test -p percpu --features sp-naive
cargo test -p percpu --features "sp-naive,non-zero-vma"
cargo test -p percpu --features non-zero-vma

# Doc tests
cargo test -p percpu --doc --features sp-naive
cargo test -p percpu --doc --features non-zero-vma
```

---

### Step 8: Commit changes

**Commands**:
```bash
git add -A
git commit -m "feat!: remove custom-base feature, unify init API

BREAKING CHANGE: custom-base feature is removed. Use init(base, cpu_count)
instead of init_dynamic(). init_static() now calls init() internally.

- Remove custom-base feature
- Rename init_dynamic to init
- Add init() to sp-naive mode
- Always use PERCPU_AREA_BASE in multi-core mode"
```

---

## Rollback Plan

If issues are found:
1. `git revert HEAD` to undo the commit
2. Or use `git reset --hard HEAD~1` if not pushed

## Success Criteria

- [ ] All tests pass for sp-naive and default modes
- [ ] Documentation is updated and accurate
- [ ] Version bumped to 0.4.0
- [ ] Changes committed to git