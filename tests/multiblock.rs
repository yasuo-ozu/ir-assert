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

fn count_nonzero(arr: &[u32]) -> usize {
    let mut count = 0;
    for &x in arr {
        if x != 0 {
            count += 1;
        }
    }
    count
}

fn sum_array(arr: &[u64]) -> u64 {
    arr.iter().copied().sum()
}

fn binary_search_manual(arr: &[i32], target: i32) -> Option<usize> {
    let mut low = 0usize;
    let mut high = arr.len();
    while low < high {
        let mid = low + (high - low) / 2;
        if arr[mid] == target {
            return Some(mid);
        } else if arr[mid] < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    None
}

#[repr(u8)]
#[allow(dead_code)]
enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
}

fn direction_to_delta(d: Direction) -> (i32, i32) {
    match d {
        Direction::North => (0, 1),
        Direction::South => (0, -1),
        Direction::East => (1, 0),
        Direction::West => (-1, 0),
    }
}

#[test]
fn test_fibonacci() {
    assert_ir!(basic_blocks.len().gt(1), fibonacci);
    assert_ir!(
        basic_blocks.len().ge(2) & basic_blocks.len().le(4),
        fibonacci
    );
    assert_ir!(branches.len().gt(0), fibonacci);
    assert_ir!(branches.len().ge(1) & branches.len().le(3), fibonacci);
    assert_ir!(phi_nodes.len().gt(0), fibonacci);
    assert_ir!(phi_nodes.len().ge(3) & phi_nodes.len().le(5), fibonacci);
    assert_ir!(calls.len().eq(0), fibonacci);
    assert_ir!(allocas.len().eq(0), fibonacci);
    assert_ir!(
        instructions.len().ge(9) & instructions.len().le(14),
        fibonacci
    );
}

#[test]
fn test_collatz() {
    assert_ir!(basic_blocks.len().gt(1), collatz_steps);
    assert_ir!(
        basic_blocks.len().ge(2) & basic_blocks.len().le(4),
        collatz_steps
    );
    assert_ir!(branches.len().gt(0), collatz_steps);
    assert_ir!(branches.len().ge(1) & branches.len().le(3), collatz_steps);
    assert_ir!(phi_nodes.len().gt(0), collatz_steps);
    assert_ir!(
        phi_nodes.len().ge(2) & phi_nodes.len().le(4),
        collatz_steps
    );
    assert_ir!(calls.len().eq(0), collatz_steps);
    assert_ir!(allocas.len().eq(0), collatz_steps);
    assert_ir!(
        instructions.len().ge(12) & instructions.len().le(18),
        collatz_steps
    );
}

#[test]
fn test_count_nonzero() {
    // Auto-vectorized loop — exact counts vary across LLVM versions
    assert_ir!(basic_blocks.len().gt(1), count_nonzero);
    assert_ir!(
        basic_blocks.len().ge(6) & basic_blocks.len().le(15),
        count_nonzero
    );
    assert_ir!(branches.len().gt(0), count_nonzero);
    assert_ir!(branches.len().ge(5) & branches.len().le(15), count_nonzero);
    assert_ir!(phi_nodes.len().gt(0), count_nonzero);
    assert_ir!(
        phi_nodes.len().ge(6) & phi_nodes.len().le(15),
        count_nonzero
    );
    assert_ir!(allocas.len().eq(0), count_nonzero);
    assert_ir!(calls.len().eq(0), count_nonzero);
    assert_ir!(
        instructions.len().ge(42) & instructions.len().le(100),
        count_nonzero
    );
}

#[test]
fn test_loop_multiple_blocks() {
    // Auto-vectorized loop — exact counts vary across LLVM versions
    assert_ir!(basic_blocks.len().gt(1), sum_array);
    assert_ir!(
        basic_blocks.len().ge(6) & basic_blocks.len().le(15),
        sum_array
    );
    assert_ir!(calls.len().eq(0), sum_array);
    assert_ir!(allocas.len().eq(0), sum_array);
    assert_ir!(branches.len().ge(5) & branches.len().le(9), sum_array);
    assert_ir!(phi_nodes.len().ge(6) & phi_nodes.len().le(10), sum_array);
    assert_ir!(
        instructions.len().ge(30) & instructions.len().le(50),
        sum_array
    );
}

#[test]
fn test_binary_search() {
    assert_ir!(basic_blocks.len().gt(1), binary_search_manual);
    assert_ir!(
        basic_blocks.len().ge(4) & basic_blocks.len().le(8),
        binary_search_manual
    );
    assert_ir!(branches.len().gt(0), binary_search_manual);
    assert_ir!(
        branches.len().ge(3) & branches.len().le(6),
        binary_search_manual
    );
    assert_ir!(phi_nodes.len().gt(0), binary_search_manual);
    assert_ir!(
        phi_nodes.len().ge(3) & phi_nodes.len().le(6),
        binary_search_manual
    );
    assert_ir!(calls.len().gt(0), binary_search_manual);
    assert_ir!(calls.len().le(2), binary_search_manual);
    assert_ir!(allocas.len().eq(0), binary_search_manual);
    assert_ir!(
        instructions.len().ge(22) & instructions.len().le(32),
        binary_search_manual
    );
}

#[test]
fn test_enum_switch_table() {
    assert_ir!(
        basic_blocks.len().eq(1) & calls.len().eq(0),
        direction_to_delta
    );
    assert_ir!(
        instructions.len().ge(7) & instructions.len().le(12),
        direction_to_delta
    );
    assert_ir!(allocas.len().eq(0), direction_to_delta);
    assert_ir!(branches.len().eq(0), direction_to_delta);
    assert_ir!(phi_nodes.len().eq(0), direction_to_delta);
}

#[test]
fn test_multiple_multiblock_targets() {
    assert_ir!(
        basic_blocks.len().gt(1) & allocas.len().eq(0),
        fibonacci,
        collatz_steps
    );
    assert_ir!(
        basic_blocks.len().ge(2) & basic_blocks.len().le(4) & allocas.len().eq(0),
        fibonacci,
        collatz_steps
    );
}
