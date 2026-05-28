# ir-assert [![Latest Version]][crates.io] [![Documentation]][docs.rs] [![GitHub Actions]][actions]

<p align="center">
  <img src="https://raw.githubusercontent.com/yasuo-ozu/ir-assert/refs/heads/main/logo.png" alt="cargo-macra logo" width="320" />
</p>

[Latest Version]: https://img.shields.io/crates/v/ir-assert.svg
[crates.io]: https://crates.io/crates/ir-assert
[Documentation]: https://img.shields.io/docsrs/ir-assert
[docs.rs]: https://docs.rs/ir-assert/latest/ir_assert/
[GitHub Actions]: https://github.com/yasuo-ozu/ir-assert/actions/workflows/test.yml/badge.svg
[actions]: https://github.com/yasuo-ozu/ir-assert/actions/workflows/test.yml

IR-level assertions for Rust tests. Verify that your code compiles to expected LLVM IR properties (e.g., a function has a single basic block, no function calls, etc.) to confirm zero-cost abstractions.

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
ir-assert = "0.1"
```

## Usage

Use `assert_ir!` inside `#[test]` functions to assert properties of the compiled LLVM IR:

```rust
use ir_assert::assert_ir;

fn add<T: std::ops::Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

#[test]
fn test_add_is_single_block_no_calls() {
    assert_ir!(
        !no_panic & basic_blocks.len().eq(1) & calls.len().eq(0),
        // function example
        add::<i32>,
        // closure example
        |a: usize, b: usize| a + b,
    );
}
```

## Predicate DSL

### Available predicates

see [predicate](https://docs.rs/ir-assert/latest/ir_assert/predicate/index.html) module documentation.

### Function-level properties

| Syntax                 | Meaning                                    |
|------------------------|--------------------------------------------|
| `basic_blocks.len()`   | Number of basic blocks in the function     |
| `calls.len()`          | Non-intrinsic call count (all blocks)      |
| `instructions.len()`   | Total instruction count across all blocks  |
| `allocas.len()`        | Number of alloca instructions              |
| `branches.len()`       | Number of br/switch instructions           |
| `phi_nodes.len()`      | Number of phi node instructions            |

### Comparison operators

All properties support: `.eq(n)`, `.ne(n)`, `.lt(n)`, `.le(n)`, `.gt(n)`, `.ge(n)`

Comparisons can be chained: `basic_blocks.len().ge(2).le(4)`

### Logical operators

- `a & b` — both must hold
- `a | b` — either must hold
- `!a` — negation
- `(a)` — grouping

### Block-level predicates

| Syntax                                          | Meaning                          |
|-------------------------------------------------|----------------------------------|
| `basic_blocks.at(N).calls().len().eq(X)`        | Calls in Nth basic block         |
| `basic_blocks.at(N).instructions().len().eq(X)` | Instructions in Nth block        |
| `basic_blocks.all(\|bb\| bb.calls.len().eq(X))` | All blocks satisfy predicate     |
| `basic_blocks.any(\|bb\| bb.calls.len().eq(X))` | Any block satisfies predicate    |

Block properties: `bb.calls`, `bb.instructions`, `bb.allocas`, `bb.branches`, `bb.phi_nodes`

### Environment predicates

Environment predicates check rustc version, target triple, or optimization level. Combine them with logical operators for conditional assertions:

```rust
#[test]
fn test_version_specific() {
    // Only assert on rustc 1.86.x
    assert_ir!(
        !rustc("1.86") | basic_blocks.len().eq(1),
        my_fn
    );

    // Only assert on specific target
    assert_ir!(
        !target("x86_64-unknown-linux-gnu") | calls.len().eq(0),
        my_fn
    );

    // target_default() always passes (tests run on host)
    assert_ir!(
        target_default() & calls.len().eq(0),
        my_fn
    );

    // Compare same code at different optimization levels on a specific target
    assert_ir!(
        (target_x86_64_unknown_linux_gnu & opt0 & instructions.len().eq(5))
            | (target_x86_64_unknown_linux_gnu & opt3 & instructions.len().eq(3)),
        my_fn
    );
}
```

| Function             | Meaning                                         |
|----------------------|-------------------------------------------------|
| `rustc("1.90")`      | Matches if rustc version equals `"1.90"`        |
| `target("triple")`   | Matches if current target equals the triple     |
| `target_default()`   | Matches default host-target environment         |
| `opt_level("0")`     | Matches optimization-level environment          |
| `opt0`..`opt3`   | Shorthand for `opt_level("0")`..`opt_level("3")` |
| `opt_s`, `opt_z` | Shorthand for `opt_level("s")`, `opt_level("z")` |


Common zero-arg target helpers:

- `target_wasm32_unknown_unknown`
- `target_x86_64_unknown_linux_gnu`
- `target_x86_64_apple_darwin`
- `target_aarch64_apple_darwin`
- `target_aarch64_unknown_linux_gnu`
- `target_x86_64_pc_windows_msvc`
- `target_aarch64_pc_windows_msvc`

## How to develop

This repository uses Rust `1.71` as MSRV, and tests environment predicates with additional toolchains.

Install required toolchains:

```bash
rustup toolchain install 1.80 1.90
```

Install required targets:

```bash
rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu
```

Then run:

```bash
cargo test --workspace --all-features
```

## How it works

1. `assert_ir!(predicate, fn1, fn2, ...)` generates a `#[no_mangle]` container function that references the target functions via inline asm (preventing optimization removal)
2. The library collects referenced environments from the predicate (`rustc(...)`, `target(...)`, `opt_level(...)`) and re-invokes `rustc`/`rustup run` to emit environment-specific `.ll` files
3. By default, IR builds use `-C opt-level=3`; `opt_level(...)` overrides this for that environment
4. The `.ll` files are parsed with a lightweight IR parser
5. The predicate is evaluated against each target function's IR in matching environments
6. If the predicate fails, the test panics with the function's IR dump for debugging

## License

MIT
