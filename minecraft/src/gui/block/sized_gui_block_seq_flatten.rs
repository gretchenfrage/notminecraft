
use crate::gui::{
    node::{
        GuiVisitorTarget,
        GuiVisitor,
    },
    block::{
        gui_block::SizedGuiBlock,
        gui_block_seq::{
            SizedGuiBlockSeq,
            GuiVisitorMaperator,
        },
    },
};


/// `SizedGuiBlock` comprising a `SizedGuiBlockSeq` and a
/// `GuiVisitorMaperator`.
///
/// Its `visit_nodes` implementation calls `visit_nodes` for each
/// `SizedGuiBlock` in the sequence, mapping `visitor` through the maperator
/// to produce each other's modified visitor.
#[derive(Debug)]
pub struct SizedGuiBlockFlatten<S, M>(pub S, pub M);

impl<
    'a,
    S: SizedGuiBlockSeq<'a>,
    M: GuiVisitorMaperator<'a>,
> SizedGuiBlock<'a> for SizedGuiBlockFlatten<S, M>
{
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
    ) {
        let SizedGuiBlockFlatten(seq, maperator) = self;
        seq.visit_items_nodes(visitor, maperator);
    }
}
