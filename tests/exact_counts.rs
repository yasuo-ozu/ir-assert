use ir_assert::assert_ir;

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

#[inline(never)]
fn noinline_add(a: u64, b: u64) -> u64 {
    a + b
}

#[inline(never)]
fn noinline_mul(a: u64, b: u64) -> u64 {
    a * b
}

#[inline(never)]
fn noinline_sub(a: u64, b: u64) -> u64 {
    a.wrapping_sub(b)
}

fn two_calls(a: u64, b: u64) -> u64 {
    let x = noinline_add(a, b);
    noinline_mul(x, b)
}

fn three_calls(a: u64, b: u64) -> u64 {
    let x = noinline_add(a, b);
    let y = noinline_mul(x, b);
    noinline_sub(y, a)
}

fn forced_two_allocas(x: u64) -> u64 {
    let mut a = [0u64; 4];
    let mut b = [0u64; 4];
    a[0] = x;
    b[0] = x + 1;
    unsafe {
        core::arch::asm!("/* {0} {1} */", in(reg) a.as_ptr(), in(reg) b.as_ptr(), options(nostack, readonly));
    }
    a[0] + b[0]
}

#[test]
fn test_exact_basic_blocks() {
    // fibonacci: entry + loop + exit = 3 basic blocks
    assert_ir!(basic_blocks.len().eq(3), fibonacci);
    // collatz_steps: entry + exit + loop = 3 basic blocks
    assert_ir!(basic_blocks.len().ge(3).le(6), collatz_steps);
}

#[test]
fn test_exact_calls() {
    // two_calls: calls noinline_add + noinline_mul = 2 calls
    assert_ir!(calls.len().eq(2), two_calls);
    // three_calls: calls noinline_add + noinline_mul + noinline_sub = 3 calls
    assert_ir!(calls.len().eq(3), three_calls);
}

#[test]
fn test_exact_allocas() {
    // forced_two_allocas: asm forces 2 stack allocations
    assert_ir!(allocas.len().eq(2), forced_two_allocas);
}

#[test]
fn test_exact_branches() {
    // fibonacci: br in entry + br in loop body = 2 branches
    assert_ir!(branches.len().eq(2), fibonacci);
    // collatz_steps: br in entry + br in loop body = 2 branches
    assert_ir!(branches.len().ge(2).le(5), collatz_steps);
}

#[test]
fn test_exact_phi_nodes() {
    // fibonacci: 3 phi in loop (a, b, i) + 1 phi in exit = 4 phi nodes
    assert_ir!(phi_nodes.len().eq(4), fibonacci);
    // collatz_steps: 2 phi in loop (n, steps) + 1 phi in exit = 3 phi nodes
    assert_ir!(phi_nodes.len().ge(3).le(4), collatz_steps);
}

#[test]
fn test_exact_instructions() {
    // two_calls: 2 tail calls + 1 ret = 3 instructions
    assert_ir!(instructions.len().eq(3), two_calls);
    // three_calls: 3 tail calls + 1 ret = 4 instructions
    assert_ir!(instructions.len().eq(4), three_calls);
}
