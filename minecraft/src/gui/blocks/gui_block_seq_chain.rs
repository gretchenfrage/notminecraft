
use crate::gui::{
    DimConstraint,
    GuiBlockSeq,
    GuiGlobalContext,
    SizedGuiBlockSeq,
    GuiVisitorTarget,
    GuiVisitorMaperator,
    GuiVisitor,
};
use std::ops::Index;


pub fn gui_chain<'a, W, H, A, B>(
    a: A,
    b: B,
) -> impl GuiBlockSeq<'a, W, H>
where
    W: DimConstraint,
    H: DimConstraint,
    A: GuiBlockSeq<'a, W, H>,
    B: GuiBlockSeq<'a, W, H>,
{
    Chain(a, b)
}


#[derive(Debug)]
struct Chain<A, B>(A, B);

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    A: GuiBlockSeq<'a, W, H>,
    B: GuiBlockSeq<'a, W, H>,
> GuiBlockSeq<'a, W, H> for Chain<A, B>
{
    type SizedSeq = Chain<A::SizedSeq, B::SizedSeq>;
    
    type WOutSeq = IndexChain<A::WOutSeq, B::WOutSeq>;

    type HOutSeq = IndexChain<A::HOutSeq, B::HOutSeq>;

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq)
    {
        let a_len = self.0.len();

        let mut w_in_seq = w_in_seq.into_iter().enumerate();
        let mut h_in_seq = h_in_seq.into_iter().enumerate();
        let mut scale_seq = scale_seq.into_iter().enumerate();

        let (a_w_out, a_h_out, a_sized) = self.0
            .size_all(
                ctx,
                (&mut w_in_seq).map(|(_, e)| e).take(a_len),
                (&mut h_in_seq).map(|(_, e)| e).take(a_len),
                (&mut scale_seq).map(|(_, e)| e).take(a_len),
            );
        let (b_w_out, b_h_out, b_sized) = self.1
            .size_all(
                ctx,
                w_in_seq.filter(|&(i, _)| i >= a_len).map(|(_, e)| e),
                h_in_seq.filter(|&(i, _)| i >= a_len).map(|(_, e)| e),
                scale_seq.filter(|&(i, _)| i >= a_len).map(|(_, e)| e),
            );

        (
            IndexChain(a_len, a_w_out, b_w_out),
            IndexChain(a_len, a_h_out, b_h_out),
            Chain(a_sized, b_sized),
        )
    }
}

impl<
    'a,
    A: SizedGuiBlockSeq<'a>,
    B: SizedGuiBlockSeq<'a>,
> SizedGuiBlockSeq<'a> for Chain<A, B>
{
    fn visit_items_nodes<T, M>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        mut maperator: M,
        forward: bool,
    )
    where
        T: GuiVisitorTarget<'a>,
        M: GuiVisitorMaperator<'a>,
    {
        if forward {
            self.0.visit_items_nodes(visitor, &mut maperator, true);
            self.1.visit_items_nodes(visitor, &mut maperator, true);
        } else {
            self.1.visit_items_nodes(visitor, &mut maperator, false);
            self.0.visit_items_nodes(visitor, &mut maperator, false);
        }
    }
}


#[derive(Debug)]
struct IndexChain<A, B>(usize, A, B);

impl<
    A: Index<usize>,
    B: Index<usize, Output=<A as Index<usize>>::Output>,
> Index<usize> for IndexChain<A, B>
{
    type Output = <A as Index<usize>>::Output;

    fn index(&self, i: usize) -> &Self::Output {
        let &IndexChain(a_len, ref a, ref b) = self;
        if i < a_len {
            &a[i]
        } else {
            &b[i - a_len]
        }
    }
}
