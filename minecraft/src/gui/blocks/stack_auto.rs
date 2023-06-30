
pub fn v_stack_auto<'a, I: GuiBlockSeq<'a, DimChildSets, DimChildSets>>(
    logical_gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {

}


#[derive(Debug)]
struct VStackAuto<I> {
    logical_gap: f32,
    items: I,
}

impl<
    'a,
    I: GuiBlockSeq<'a, DimChildSets, DimChildSets>,
> GuiBlock<'a, DimChildSets, DimChildSets> for VStackAuto<I> {
    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized)
    {
        let len = self.items.len();

        let scaled_gap = self.logical_gap * scale;
    }
}
