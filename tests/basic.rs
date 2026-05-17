use ir_assert::assert_ir;

fn f1<T>(a: T) -> T {
    a
}

fn add_three<T: std::ops::Add<Output = T>>(a: T, b: T, c: T) -> T {
    a + b + c
}

fn identity(x: u64) -> u64 {
    x
}

fn bitwise_or(a: u32, b: u32) -> u32 {
    a | b
}

fn max(a: i64, b: i64) -> i64 {
    if a > b {
        a
    } else {
        b
    }
}

fn polynomial(x: i64) -> i64 {
    x * x * x + 3 * x * x + 2 * x + 1
}

#[test]
fn test_basic_single_block() {
    // f1::<i32>: bb=1, calls=0, instr=1, allocas=0, branches=0, phi=0
    assert_ir!(basic_blocks.len().eq(1), f1::<i32>);
    assert_ir!(calls.len().eq(0), f1::<i32>);
    assert_ir!(instructions.len().eq(1), f1::<i32>);
    // |a,b| a+b: bb=1, calls=0, instr=2, allocas=0, branches=0, phi=0
    assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a: usize, b: usize| a + b
    );
    assert_ir!(instructions.len().eq(2), |a: usize, b: usize| a + b);
    assert_ir!(
        allocas.len().eq(0) & branches.len().eq(0) & phi_nodes.len().eq(0),
        |a: usize, b: usize| a + b
    );
    // add_three::<i32>: bb=1, calls=0, instr=3, allocas=0, branches=0, phi=0
    assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        add_three::<i32>
    );
    assert_ir!(instructions.len().eq(3), add_three::<i32>);
    assert_ir!(
        allocas.len().eq(0) & branches.len().eq(0) & phi_nodes.len().eq(0),
        add_three::<i32>
    );
}

#[test]
fn test_multiple_targets() {
    assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        f1::<u64>,
        identity,
        bitwise_or
    );
}

#[test]
fn test_comparison_operators() {
    assert_ir!(basic_blocks.len().ge(1), f1::<i32>);
    assert_ir!(basic_blocks.len().le(1), f1::<i32>);
    assert_ir!(basic_blocks.len().ne(0), f1::<i32>);
    assert_ir!(calls.len().lt(1), f1::<i32>);
    assert_ir!(instructions.len().gt(0), f1::<i32>);
}

#[test]
fn test_negation() {
    assert_ir!(!(calls.len().gt(0)), f1::<i32>);
    assert_ir!(calls.len().eq(0), f1::<i32>);
}

#[test]
fn test_or_combinator() {
    assert_ir!(
        basic_blocks.len().eq(1) | basic_blocks.len().eq(2),
        f1::<i32>
    );
}

#[test]
fn test_max_optimized() {
    // max: bb=1, calls=0, instr=2, allocas=0, branches=0, phi=0
    assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), max);
    assert_ir!(instructions.len().eq(2), max);
    assert_ir!(allocas.len().eq(0), max);
    assert_ir!(phi_nodes.len().eq(0), max);
}

#[test]
fn test_no_allocas() {
    assert_ir!(allocas.len().eq(0), f1::<i32>);
    assert_ir!(allocas.len().eq(0), identity);
}

#[test]
fn test_branches() {
    assert_ir!(branches.len().eq(0), f1::<i32>);
    assert_ir!(branches.len().eq(0), max);
    assert_ir!(branches.len().eq(0), max);
}

#[test]
fn test_phi_nodes() {
    assert_ir!(phi_nodes.len().eq(0), f1::<i32>);
}

#[test]
fn test_polynomial() {
    // polynomial: bb=1, calls=0, instr=6, allocas=0, branches=0, phi=0
    assert_ir!(
        basic_blocks.len().eq(1)
            & phi_nodes.len().eq(0)
            & branches.len().eq(0)
            & calls.len().eq(0),
        polynomial
    );
    assert_ir!(instructions.len().eq(6), polynomial);
    assert_ir!(allocas.len().eq(0), polynomial);
}

#[test]
fn test_add_three_exact() {
    assert_ir!(instructions.len().eq(3), add_three::<i32>);
    assert_ir!(allocas.len().eq(0), add_three::<i32>);
    assert_ir!(branches.len().eq(0), add_three::<i32>);
    assert_ir!(phi_nodes.len().eq(0), add_three::<i32>);
}

#[test]
fn test_identity_exact() {
    // identity: bb=1, calls=0, instr=1, allocas=0, branches=0, phi=0
    assert_ir!(basic_blocks.len().eq(1), identity);
    assert_ir!(calls.len().eq(0), identity);
    assert_ir!(instructions.len().eq(1), identity);
    assert_ir!(branches.len().eq(0), identity);
    assert_ir!(phi_nodes.len().eq(0), identity);
}

#[test]
fn test_add_closure_exact() {
    assert_ir!(instructions.len().eq(2), |a: usize, b: usize| a + b);
    assert_ir!(allocas.len().eq(0), |a: usize, b: usize| a + b);
    assert_ir!(branches.len().eq(0), |a: usize, b: usize| a + b);
    assert_ir!(phi_nodes.len().eq(0), |a: usize, b: usize| a + b);
}

#[test]
fn test_bitwise_or_exact() {
    // bitwise_or: bb=1, calls=0, instr=2, allocas=0, branches=0, phi=0
    assert_ir!(basic_blocks.len().eq(1), bitwise_or);
    assert_ir!(calls.len().eq(0), bitwise_or);
    assert_ir!(instructions.len().eq(2), bitwise_or);
    assert_ir!(allocas.len().eq(0), bitwise_or);
    assert_ir!(branches.len().eq(0), bitwise_or);
    assert_ir!(phi_nodes.len().eq(0), bitwise_or);
}
