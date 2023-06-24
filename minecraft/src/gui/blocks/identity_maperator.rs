
use crate::gui::{
    GuiVisitorMaperator,
    GuiVisitorTarget,
    GuiVisitor,
};


#[derive(Debug)]
pub struct IdentityMaperator;

impl<'a> GuiVisitorMaperator<'a> for IdentityMaperator {
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        visitor.reborrow()
    }
}
