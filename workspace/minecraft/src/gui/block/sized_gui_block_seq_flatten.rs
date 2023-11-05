
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
use std::fmt::Debug;


/// A direction -> `GuiVisitorMaperator` constructor.
pub trait DirMaperators<'a>: Debug {
    type Forward: GuiVisitorMaperator<'a>;
    type Reverse: GuiVisitorMaperator<'a>;

    fn forward(self) -> Self::Forward;
    fn reverse(self) -> Self::Reverse;
}

/// Directionally symmetric maperator. Adapts `GuiVisitorMaperator` to
/// `DirMaperators` by simply returning the same inner value regardless of
/// direction.
#[derive(Debug)]
pub struct DirSymMaperator<M>(pub M);

impl<
    'a,
    M: GuiVisitorMaperator<'a>,
> DirMaperators<'a> for DirSymMaperator<M>
{
    type Forward = M;
    type Reverse = M;

    fn forward(self) -> Self::Forward { self.0 }
    fn reverse(self) -> Self::Reverse { self.0 }
}


/// `SizedGuiBlock` comprising a `SizedGuiBlockSeq` and a direction ->
/// `GuiVisitorMaperator` constructor.
///
/// Its `visit_nodes` implementation calls `visit_nodes` for each
/// `SizedGuiBlock` in the sequence, mapping `visitor` through the maperator
/// to produce each other's modified visitor.
#[derive(Debug)]
pub struct SizedGuiBlockFlatten<S, M>(pub S, pub M);

impl<
    'a,
    S: SizedGuiBlockSeq<'a>,
    M: DirMaperators<'a>,
> SizedGuiBlock<'a> for SizedGuiBlockFlatten<S, M>
{
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let SizedGuiBlockFlatten(seq, maperators) = self;
        if forward {
            let maperator = maperators.forward();
            seq.visit_items_nodes(visitor, maperator, true);
        } else {
            let maperator = maperators.reverse();
            seq.visit_items_nodes(visitor, maperator, false);
        }
    }
}
