# Implementation Plan: percpu Feature Modes

Based on the design document: `docs/plans/2026-03-06-feature-modes-design.md`

## Task Breakdown

### Task 1: Update naive.rs init() signature for custom-base compatibility

**File:** `percpu/src/naive.rs`

**Changes:**
- Add conditional compilation for `init()` function
- When `custom-base` is enabled, accept `base` and `cpu_count` parameters (but ignore them)
- Add documentation explaining the behavior

**Acceptance criteria:**
- `init()` compiles with `sp-naive` alone
- `init(base, count)` compiles with `sp-naive,custom-base`
- Parameters are ignored in both cases
- Function returns `1` in all cases

---

### Task 2: Update test file cfg attributes

**File:** `percpu/tests/test_percpu.rs`

**Changes:**
- Update the test function's `#[cfg]` attribute from:
  ```rust
  #[cfg(all(target_os = "linux", feature = "non-zero-vma"))]
  ```
  to:
  ```rust
  #[cfg(all(target_os = "linux", any(feature = "non-zero-vma", feature = "sp-naive")))]
  ```
- Update `test_remote_access` cfg to exclude `sp-naive`:
  ```rust
  #[cfg(all(target_os = "linux", not(feature = "sp-naive"), feature = "non-zero-vma"))]
  ```

**Acceptance criteria:**
- Test compiles and runs with `sp-naive` alone
- Test compiles and runs with `sp-naive,custom-base`
- Test compiles and runs with `non-zero-vma`
- Test compiles and runs with `custom-base,non-zero-vma`

---

### Task 3: Refactor test initialization logic

**File:** `percpu/tests/test_percpu.rs`

**Changes:**
- Extract initialization logic into a helper function `setup_percpu() -> usize`
- The helper should handle all 4 test configurations:
  - `sp-naive` (with or without `custom-base`): returns 0
  - `non-zero-vma`: calls `init()`, sets up per-CPU register
  - `custom-base,non-zero-vma`: allocates memory, calls `init(base, count)`
- Update test function to use the helper
- Keep existing test assertions unchanged

**Acceptance criteria:**
- All 4 test configurations pass
- Test code is more maintainable
- Shared logic is not duplicated

---

### Task 4: Update test.yml CI workflow

**File:** `.github/workflows/test.yml`

**Changes:**
- Add test configuration for `sp-naive`:
  ```yaml
  - name: Run tests (sp-naive)
    run: cargo test --target x86_64-unknown-linux-gnu --features "sp-naive" -- --nocapture
  ```
- Add test configuration for `sp-naive,custom-base`:
  ```yaml
  - name: Run tests (sp-naive,custom-base)
    run: cargo test --target x86_64-unknown-linux-gnu --features "sp-naive,custom-base" -- --nocapture
  ```
- Add test configuration for `custom-base,non-zero-vma`:
  ```yaml
  - name: Run tests (custom-base,non-zero-vma)
    run: cargo test --target x86_64-unknown-linux-gnu --features "custom-base,non-zero-vma" -- --nocapture
  ```
- Keep existing `non-zero-vma` and `sp-naive,non-zero-vma` tests

**Acceptance criteria:**
- CI runs 5 test configurations (or 4 if `sp-naive,non-zero-vma` is redundant with `sp-naive`)
- All tests pass

---

### Task 5: Update check.yml CI workflow

**File:** `.github/workflows/check.yml`

**Changes:**
- Add clippy/build steps for `custom-base`:
  ```yaml
  - name: Clippy (custom-base)
    run: cargo clippy --target ${{ matrix.target }} --features "custom-base"

  - name: Build (custom-base)
    run: cargo build --target ${{ matrix.target }} --features "custom-base"
  ```
- Add clippy/build steps for `custom-base,non-zero-vma`:
  ```yaml
  - name: Clippy (custom-base,non-zero-vma)
    run: cargo clippy --target ${{ matrix.target }} --features "custom-base,non-zero-vma"

  - name: Build (custom-base,non-zero-vma)
    run: cargo build --target ${{ matrix.target }} --features "custom-base,non-zero-vma"
  ```

**Acceptance criteria:**
- CI builds all feature combinations for all targets
- No clippy warnings

---

### Task 6: Update README documentation

**File:** `README.md`

**Changes:**
- Add a new "Feature Modes" section with a table of all modes
- Add a "Bare Metal vs Linux" section explaining environment differences
- Add a "Why Each Mode Exists" subsection
- Add a note about `sp-naive` interactions with `custom-base` and `non-zero-vma`
- Update the example code to show different initialization patterns

**Content structure:**
```markdown
## Feature Modes

| Mode | Features | `init()` | Memory | VMA | Bare Metal | Linux |
|------|----------|----------|--------|-----|------------|-------|
| Default | (none) | `init()` | Linker | 0 | ✅ | ❌ |
| sp-naive | `sp-naive` | `init(base?, count?)` | Global | N/A | ✅ | ✅ |
| non-zero-vma | `non-zero-vma` | `init()` | Linker | Any | ✅ | ✅ |
| custom-base | `custom-base` | `init(base, count)` | User | 0 | ✅ | ❌ |
| custom-base+non-zero-vma | `custom-base,non-zero-vma` | `init(base, count)` | User | Any | ✅ | ✅ |

### Bare Metal vs Linux

[differences table and explanations]

### Why Each Mode Exists

[explanations for each mode]

### sp-naive Feature Interactions

[explanation of how custom-base and non-zero-vma are accepted but ignored]
```

**Acceptance criteria:**
- README clearly documents all modes
- Users can understand which mode to use for their scenario
- Environment differences are clear

---

## Execution Order

1. Task 1 (naive.rs) - Required first for tests to compile
2. Task 2 (test cfg) - Depends on Task 1
3. Task 3 (test refactor) - Depends on Task 2
4. Task 4 (test.yml) - Can run after Task 3
5. Task 5 (check.yml) - Independent
6. Task 6 (README) - Independent

## Verification

After all tasks:
1. Run `cargo test` with each test configuration locally
2. Run `cargo clippy` with all feature combinations
3. Verify README renders correctly
4. Push changes and verify CI passes