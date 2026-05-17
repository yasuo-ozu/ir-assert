use ir_assert::assert_ir;

fn main() {
    assert_ir!(basic_blocks.len().ge(1), |a, b| a + b);
}
