use ir_assert::assert_ir;

fn add(a: u64, b: u64) -> u64 {
    a + b
}

fn identity(x: u64) -> u64 {
    x
}

#[test]
fn test_return_named_function() {
    let f = assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), add);
    assert_eq!(f(3, 4), 7);
}

#[test]
fn test_return_generic_function() {
    fn generic_add<T: std::ops::Add<Output = T>>(a: T, b: T) -> T {
        a + b
    }
    let f = assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), generic_add::<i32>);
    assert_eq!(f(10, 20), 30);
}

#[test]
fn test_return_closure() {
    let f = assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a: usize, b: usize| a + b
    );
    assert_eq!(f(5, 6), 11);
}

#[test]
fn test_return_closure_single_arg() {
    let f = assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), |x: u32| x * 2);
    assert_eq!(f(7), 14);
}

#[test]
fn test_return_identity() {
    let f = assert_ir!(basic_blocks.len().eq(1), identity);
    assert_eq!(f(42), 42);
}

#[test]
fn test_return_closure_untyped_args() {
    // Closure params have no type annotations; types are inferred from the call site.
    // The container uses `usize` as the fallback type for IR checking.
    let f = assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a, b| a + b
    );
    assert_eq!(f(10u64, 20u64), 30u64);
}

#[test]
fn test_return_closure_untyped_single_arg() {
    let f = assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), |x| x * 3);
    assert_eq!(f(5u32), 15u32);
}

#[test]
fn test_return_closure_untyped_three_args() {
    let f = assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a, b, c| a + b + c
    );
    assert_eq!(f(1i64, 2i64, 3i64), 6i64);
}

#[test]
fn test_multiple_targets_returns_unit() {
    // With multiple targets, the macro returns () — just verify it compiles
    let _: () = assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), add, identity);
}
