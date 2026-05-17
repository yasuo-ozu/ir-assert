use ir_assert::assert_ir;

fn complex_callee(mut x: u64) -> u64 {
    let mut acc = 0u64;
    let mut i = 0u64;
    while i < 4 {
        acc = acc.wrapping_add((x ^ (i * 17)).rotate_left((i + 3) as u32));
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        i += 1;
    }
    acc ^ x
}

fn complex_caller(a: u64, b: u64) -> u64 {
    let mut x = a ^ b.rotate_left(5);
    let mut i = 0u64;
    while i < a {
        let t = x.wrapping_add(i.wrapping_mul(17));
        if (t & 1) == 0 {
            x = x.wrapping_add(complex_callee(t));
        } else {
            x = x.wrapping_add(complex_callee(t ^ 0xa5a5_a5a5));
        }
        i += 1;
    }
    x
}

fn branch_mix(x: u64) -> u64 {
    let mut acc = 0u64;
    let mut i = 0u64;
    while i < 6 {
        let v = x.wrapping_add(i.wrapping_mul(13));
        if (v & 1) == 0 {
            acc = acc.wrapping_add(v.rotate_left((i + 1) as u32));
        } else {
            acc = acc.wrapping_add(v.wrapping_mul(3).wrapping_add(1));
        }
        i += 1;
    }
    acc
}

fn sum_shifted_array(x: u64, y: u64) -> u64 {
    let mut m = branch_mix(x ^ y.rotate_left(7));
    if x > y {
        m = m.wrapping_add(1);
    }
    let values = [
        m,
        m.wrapping_add(1),
        m.wrapping_add(2),
        m.wrapping_add(3),
        m.wrapping_add(5),
        m.wrapping_add(8),
    ];
    let mut i = 0usize;
    let mut acc = 0u64;
    while i < values.len() {
        acc = acc.wrapping_add(values[i]);
        i += 1;
    }
    acc
}

#[test]
fn test_calls_len_opt0_and_opt3() {
    assert_ir!(opt0 & calls.len().ge(1), complex_caller);
    assert_ir!(opt3 & calls.len().eq(0), complex_caller);
}

#[test]
fn test_basic_blocks_opt0_and_opt3() {
    // At opt0 branch_mix is not inlined and the loop is preserved.
    assert_ir!(opt0 & basic_blocks.len().ge(2), sum_shifted_array);
    // At opt3 the loop is unrolled; newer LLVM collapses to 1 block,
    // older LLVM (e.g. Rust 1.71) keeps ~19 blocks from branch_mix inlining.
    assert_ir!(opt3 & calls.len().eq(0), sum_shifted_array);
}
