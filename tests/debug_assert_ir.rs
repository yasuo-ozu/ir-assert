use ir_assert::debug_assert_ir;

fn add(a: u64, b: u64) -> u64 {
    a + b
}

fn identity(x: u64) -> u64 {
    x
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn generic_add<T: std::ops::Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

// --- single named function: IR checked in debug, passthru in release ---

#[test]
fn test_single_named_fn_returns_callable() {
    let f = debug_assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), add);
    assert_eq!(f(3, 4), 7);
}

#[test]
fn test_single_named_fn_identity() {
    let f = debug_assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), identity);
    assert_eq!(f(42), 42);
}

#[test]
fn test_single_named_fn_multiply() {
    let f = debug_assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), multiply);
    assert_eq!(f(6, 7), 42);
}

// --- single generic function ---

#[test]
fn test_single_generic_fn() {
    let f = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        generic_add::<i64>
    );
    assert_eq!(f(10, 20), 30);
}

// --- single closure ---

#[test]
fn test_single_closure_typed_args() {
    let f = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a: usize, b: usize| a + b
    );
    assert_eq!(f(5, 6), 11);
}

#[test]
fn test_single_closure_single_arg() {
    let f = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |x: u32| x * 2
    );
    assert_eq!(f(7), 14);
}

#[test]
fn test_single_closure_untyped_args() {
    let f = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a, b| a + b
    );
    assert_eq!(f(10u64, 20u64), 30u64);
}

// --- multiple targets (debug mode only: IR checked, returns ()) ---

#[test]
fn test_multi_target_named_fns() {
    let _: () = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        add,
        identity
    );
}

#[test]
fn test_multi_target_mixed_closure_and_fn() {
    let _: () = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        add,
        |a: usize, b: usize| a + b
    );
}

#[test]
fn test_multi_target_three_fns() {
    let _: () = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        add,
        identity,
        multiply
    );
}

// --- verify predicate is enforced in debug mode ---

#[test]
#[should_panic(expected = "ir-assert: assertion failed")]
fn test_failing_predicate_panics_in_debug() {
    // identity has 1 basic block; requiring 99 must fail.
    // debug_assert_ir! skips the assertion in release mode so this test only
    // makes sense under `cargo test` (which uses debug_assertions=true).
    debug_assert_ir!(basic_blocks.len().eq(99), identity);
}

// --- passthru: in release mode the returned value still has the right type ---

#[test]
fn test_return_type_is_correct() {
    // Regardless of debug_assertions, the macro returns a callable fn pointer.
    let f: fn(u64, u64) -> u64 = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        add
    );
    assert_eq!(f(1, 1), 2);
}

#[test]
fn test_return_closure_type_is_correct() {
    let f: fn(u32) -> u32 = debug_assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |x: u32| x * 3
    );
    assert_eq!(f(5), 15);
}
