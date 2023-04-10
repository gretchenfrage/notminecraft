
use crate::gui::*;
use std::fmt::{
    self,
    Formatter,
    Debug,
};


struct ArrayMapGuiBlockSeq<A, M> {
    array: A,
    map: M,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: Debug,
    M: FnMut(I) -> B,
    B: GuiBlock<'a, W, H>,
    const LEN: usize,
> GuiBlockSeq<'a, W, H> for ArrayMapGuiBlockSeq<[I; LEN], M> {
    type SizedSeq = [B::Sized; LEN];
    type WOutSeq = [W::Out; LEN];
    type HOutSeq = [H::Out; LEN];

    fn len(&self) -> usize { LEN }

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        mut self,
        ctx: &GuiGlobalContext<'a>,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
        let mut w_in_iter = w_in_seq.into_iter();
        let mut h_in_iter = h_in_seq.into_iter();
        let mut scale_iter = scale_seq.into_iter();

        let mut w_out_seq = [W::Out::default(); LEN];
        let mut h_out_seq = [H::Out::default(); LEN];
        let mut i = 0;

        let sized_seq = self.array.map(|item| {
            let block = (self.map)(item);
            let (w_out, h_out, sized) = block.size(
                ctx,
                w_in_iter.next().unwrap(),
                h_in_iter.next().unwrap(),
                scale_iter.next().unwrap(),
            );

            w_out_seq[i] = w_out;
            h_out_seq[i] = h_out;
            i += 1;
            
            sized
        });

        (w_out_seq, h_out_seq, sized_seq)
    }
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: Debug,
    M: FnMut(&'a I) -> B,
    B: GuiBlock<'a, W, H>,
    const LEN: usize,
> GuiBlockSeq<'a, W, H> for ArrayMapGuiBlockSeq<&'a [I; LEN], M> {
    type SizedSeq = [B::Sized; LEN];
    type WOutSeq = [W::Out; LEN];
    type HOutSeq = [H::Out; LEN];

    fn len(&self) -> usize { LEN }

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        mut self,
        ctx: &GuiGlobalContext<'a>,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
        let mut w_in_iter = w_in_seq.into_iter();
        let mut h_in_iter = h_in_seq.into_iter();
        let mut scale_iter = scale_seq.into_iter();

        let mut w_out_seq = [W::Out::default(); LEN];
        let mut h_out_seq = [H::Out::default(); LEN];
        let mut i = 0;

        let sized_seq = self.array.map(|item| {
            let block = (self.map)(item);
            let (w_out, h_out, sized) = block.size(
                ctx,
                w_in_iter.next().unwrap(),
                h_in_iter.next().unwrap(),
                scale_iter.next().unwrap(),
            );

            w_out_seq[i] = w_out;
            h_out_seq[i] = h_out;
            i += 1;
            
            sized
        });

        (w_out_seq, h_out_seq, sized_seq)
    }
}

impl<A: Debug, M> Debug for ArrayMapGuiBlockSeq<A, M> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ArrayMapGuiBlockSeq")
            .field("array", &self.array)
            .finish_non_exhaustive()
    }
}
