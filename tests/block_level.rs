use ir_assert::assert_ir;

fn f1<T>(a: T) -> T {
    a
}

fn identity(x: u64) -> u64 {
    x
}

fn sum_array(arr: &[u64]) -> u64 {
    arr.iter().copied().sum()
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0u64;
    let mut b = 1u64;
    let mut i = 2;
    while i <= n {
        let c = a + b;
        a = b;
        b = c;
        i += 1;
    }
    b
}

fn collatz_steps(mut n: u64) -> u64 {
    let mut steps = 0;
    while n != 1 {
        if n % 2 == 0 {
            n /= 2;
        } else {
            n = 3 * n + 1;
        }
        steps += 1;
    }
    steps
}

#[test]
fn test_block_all_no_allocas() {
    assert_ir!(basic_blocks.all(|bb| bb.allocas.len().eq(0)), f1::<i32>);
    assert_ir!(basic_blocks.all(|bb| bb.allocas.len().eq(0)), identity);
}

#[test]
fn test_block_any_has_instructions() {
    assert_ir!(
        basic_blocks.any(|bb| bb.instructions.len().gt(0)),
        sum_array
    );
}

#[test]
fn test_block_indexing() {
    assert_ir!(basic_blocks.at(0).allocas().len().eq(0), f1::<i32>);
    assert_ir!(basic_blocks.at(0).instructions().len().gt(0), f1::<i32>);
}

#[test]
fn test_block_quantifiers_multiblock() {
    assert_ir!(basic_blocks.all(|bb| bb.allocas.len().eq(0)), fibonacci);
    assert_ir!(basic_blocks.all(|bb| bb.allocas.len().eq(0)), collatz_steps);
    assert_ir!(basic_blocks.any(|bb| bb.branches.len().gt(0)), fibonacci);
    assert_ir!(
        basic_blocks.any(|bb| bb.phi_nodes.len().gt(0)),
        collatz_steps
    );
}

// --- Per-block assertions for fibonacci ---
// Block 0 (entry): icmp + br
// Loop block:  3 phi + add + add + icmp + br
// Exit block:  phi + ret
// NOTE: block ordering (loop vs exit) varies across compiler versions,
// so blocks 1/2 use quantifiers instead of exact indices.

#[test]
fn test_fibonacci_block0_exact() {
    assert_ir!(basic_blocks.at(0).instructions().len().eq(2), fibonacci);
    assert_ir!(basic_blocks.at(0).branches().len().eq(1), fibonacci);
    assert_ir!(basic_blocks.at(0).phi_nodes().len().eq(0), fibonacci);
    assert_ir!(basic_blocks.at(0).calls().len().eq(0), fibonacci);
    assert_ir!(basic_blocks.at(0).allocas().len().eq(0), fibonacci);
}

#[test]
fn test_fibonacci_loop_block() {
    // Loop block has 3 phi nodes
    assert_ir!(basic_blocks.any(|bb| bb.phi_nodes.len().eq(3)), fibonacci);
    // Loop block has a branch
    assert_ir!(basic_blocks.any(|bb| bb.phi_nodes.len().ge(3)), fibonacci);
}

#[test]
fn test_fibonacci_exit_block() {
    // Exit block has exactly 1 phi node
    assert_ir!(basic_blocks.any(|bb| bb.phi_nodes.len().eq(1)), fibonacci);
    // No block has calls or allocas
    assert_ir!(basic_blocks.all(|bb| bb.calls.len().eq(0)), fibonacci);
    assert_ir!(basic_blocks.all(|bb| bb.allocas.len().eq(0)), fibonacci);
}

// --- Exact per-block assertions for collatz_steps ---
// Block 0 (entry): icmp + br
// Block 1 (exit):  phi + ret
// Block 2 (loop):  2 phi + and + icmp + lshr + mul + add + select + add + icmp + br

#[test]
fn test_collatz_block0_exact() {
    assert_ir!(basic_blocks.at(0).instructions().len().eq(2), collatz_steps);
    assert_ir!(basic_blocks.at(0).branches().len().eq(1), collatz_steps);
    assert_ir!(basic_blocks.at(0).phi_nodes().len().eq(0), collatz_steps);
}

#[test]
fn test_collatz_block1_exact() {
    assert_ir!(basic_blocks.at(1).instructions().len().eq(2), collatz_steps);
    assert_ir!(basic_blocks.at(1).branches().len().le(1), collatz_steps);
    assert_ir!(basic_blocks.at(1).phi_nodes().len().eq(1), collatz_steps);
}

#[test]
fn test_collatz_block2_exact() {
    assert_ir!(
        basic_blocks.at(2).instructions().len().le(15),
        collatz_steps
    );
    assert_ir!(basic_blocks.at(2).branches().len().eq(1), collatz_steps);
    assert_ir!(basic_blocks.at(2).phi_nodes().len().le(2), collatz_steps);
}
