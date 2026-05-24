use super::combinators::{At, BlockPred, Cmp, Quantifier};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy)]
pub enum Property {
    BasicBlocksLen,
    CallsLen,
    InstructionsLen,
    AllocasLen,
    BranchesLen,
    PhiNodesLen,
}

impl Property {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Property::BasicBlocksLen => "basic_blocks.len()",
            Property::CallsLen => "calls.len()",
            Property::InstructionsLen => "instructions.len()",
            Property::AllocasLen => "allocas.len()",
            Property::BranchesLen => "branches.len()",
            Property::PhiNodesLen => "phi_nodes.len()",
        }
    }
}

macro_rules! define_block_accessors {
    ($Acc:ident { $($method:ident => $variant:ident),+ $(,)? }) => {
        $(
            pub fn $method(self) -> $Acc {
                $Acc { index: self.0, property: Property::$variant }
            }
        )+
    };
}

// --- DSL builder types ---

pub struct PropertyAccess(pub Property);

impl PropertyAccess {
    pub fn len(&self) -> PropertyLen {
        PropertyLen(self.0)
    }
}

pub struct PropertyLen(Property);

impl PropertyLen {
    super::define_cmp_methods!(Cmp, |self, op| Cmp {
        property: self.0,
        op
    });
}

// --- BasicBlocks entry point ---

pub struct BasicBlocks;

impl BasicBlocks {
    pub fn len(&self) -> PropertyLen {
        PropertyLen(Property::BasicBlocksLen)
    }

    pub fn all(&self, f: impl Fn(BlockRef) -> BlockPred + 'static) -> Quantifier {
        Quantifier {
            require_all: true,
            f: Box::new(f),
        }
    }

    pub fn any(&self, f: impl Fn(BlockRef) -> BlockPred + 'static) -> Quantifier {
        Quantifier {
            require_all: false,
            f: Box::new(f),
        }
    }

    pub fn at(&self, index: usize) -> IndexedBlockRef {
        IndexedBlockRef(index)
    }
}

// --- BlockRef (used inside .all()/.any() closures) ---

pub struct BlockRef {
    pub calls: BlockPropertyAccess,
    pub instructions: BlockPropertyAccess,
    pub allocas: BlockPropertyAccess,
    pub branches: BlockPropertyAccess,
    pub phi_nodes: BlockPropertyAccess,
}

impl BlockRef {
    pub fn new() -> Self {
        BlockRef {
            calls: BlockPropertyAccess(Property::CallsLen),
            instructions: BlockPropertyAccess(Property::InstructionsLen),
            allocas: BlockPropertyAccess(Property::AllocasLen),
            branches: BlockPropertyAccess(Property::BranchesLen),
            phi_nodes: BlockPropertyAccess(Property::PhiNodesLen),
        }
    }
}

impl Default for BlockRef {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockPropertyAccess(Property);

impl BlockPropertyAccess {
    pub fn len(&self) -> BlockPropertyLen {
        BlockPropertyLen(self.0)
    }
}

pub struct BlockPropertyLen(Property);

impl BlockPropertyLen {
    super::define_cmp_methods!(BlockPred, |self, op| BlockPred::Cmp {
        property: self.0,
        op
    });
}

// --- IndexedBlockRef (for basic_blocks.at(N)) ---

pub struct IndexedBlockRef(pub usize);

impl IndexedBlockRef {
    define_block_accessors!(IndexedBlockPropertyAccess {
        calls => CallsLen,
        instructions => InstructionsLen,
        allocas => AllocasLen,
        branches => BranchesLen,
        phi_nodes => PhiNodesLen,
    });
}

pub struct IndexedBlockPropertyAccess {
    index: usize,
    property: Property,
}

impl IndexedBlockPropertyAccess {
    pub fn len(self) -> IndexedBlockPropertyLen {
        IndexedBlockPropertyLen {
            index: self.index,
            property: self.property,
        }
    }
}

pub struct IndexedBlockPropertyLen {
    index: usize,
    property: Property,
}

impl IndexedBlockPropertyLen {
    super::define_cmp_methods!(At, |self, op| At {
        index: self.index,
        pred: BlockPred::Cmp {
            property: self.property,
            op
        },
    });
}
