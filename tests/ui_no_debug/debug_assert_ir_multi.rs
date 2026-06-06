use ir_assert::debug_assert_ir;

fn add(a: u64, b: u64) -> u64 {
    a + b
}

fn identity(x: u64) -> u64 {
    x
}

fn main() {
    debug_assert_ir!(basic_blocks.len().eq(1) & calls.len().eq(0), add, identity);
}
