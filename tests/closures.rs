use ir_assert::assert_ir;

#[test]
fn test_closure_single_arg() {
    assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), |x: u32| x
        * 2);
    assert_ir!(instructions.len().eq(2), |x: u32| x * 2);
    assert_ir!(allocas.len().eq(0), |x: u32| x * 2);
    assert_ir!(branches.len().eq(0), |x: u32| x * 2);
    assert_ir!(phi_nodes.len().eq(0), |x: u32| x * 2);
}

#[test]
fn test_closure_complex() {
    assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        |a: i64, b: i64, c: i64| a + b * c
    );
    assert_ir!(instructions.len().eq(3), |a: i64, b: i64, c: i64| a + b * c);
    assert_ir!(allocas.len().eq(0), |a: i64, b: i64, c: i64| a + b * c);
    assert_ir!(branches.len().eq(0), |a: i64, b: i64, c: i64| a + b * c);
    assert_ir!(phi_nodes.len().eq(0), |a: i64, b: i64, c: i64| a + b * c);
}

#[test]
fn test_closure_with_loop() {
    assert_ir!(
        basic_blocks.len().gt(1) & branches.len().gt(0) & phi_nodes.len().gt(0),
        |n: u64| -> u64 {
            let mut acc = 0u64;
            let mut i = 0u64;
            while i < n {
                acc += i;
                i += 1;
            }
            acc
        }
    );
    assert_ir!(
        basic_blocks.len().eq(3) & calls.len().eq(0),
        |n: u64| -> u64 {
            let mut acc = 0u64;
            let mut i = 0u64;
            while i < n {
                acc += i;
                i += 1;
            }
            acc
        }
    );
    assert_ir!(
        branches.len().eq(2) & phi_nodes.len().eq(1),
        |n: u64| -> u64 {
            let mut acc = 0u64;
            let mut i = 0u64;
            while i < n {
                acc += i;
                i += 1;
            }
            acc
        }
    );
    assert_ir!(allocas.len().eq(0), |n: u64| -> u64 {
        let mut acc = 0u64;
        let mut i = 0u64;
        while i < n {
            acc += i;
            i += 1;
        }
        acc
    });
}

#[test]
fn test_closure_branchless() {
    assert_ir!(
        basic_blocks.len().eq(1) & branches.len().eq(0),
        |x: i32, lo: i32, hi: i32| -> i32 {
            if x < lo {
                lo
            } else if x > hi {
                hi
            } else {
                x
            }
        }
    );
    assert_ir!(
        calls.len().eq(0) & allocas.len().eq(0) & phi_nodes.len().eq(0),
        |x: i32, lo: i32, hi: i32| -> i32 {
            if x < lo {
                lo
            } else if x > hi {
                hi
            } else {
                x
            }
        }
    );
    assert_ir!(instructions.len().eq(4), |x: i32,
                                          lo: i32,
                                          hi: i32|
     -> i32 {
        if x < lo {
            lo
        } else if x > hi {
            hi
        } else {
            x
        }
    });
}
