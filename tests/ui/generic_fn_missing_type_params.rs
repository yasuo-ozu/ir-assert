use ir_assert::assert_ir;

fn id<T>(x: T) -> T {
    x
}

fn main() {
    assert_ir!(basic_blocks.len().ge(1), id);
}
