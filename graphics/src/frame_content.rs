
use crate::{
    modifier::{
        Modifier2,
        Modifier3,
        Transform2,
        Transform3,
        Clip2,
        Clip3,
    },
    view_proj::ViewProj,
};
use std::collections::VecDeque;
use vek::*;


#[derive(Default)]
pub struct FrameContent {
    instrs: Vec<(usize, DrawInstr)>,
}

enum DrawInstr {
    PushModifier2(Modifier2),
    Draw2(DrawObj2),
    Begin3d(ViewProj),
    PushModifier3(Modifier3),
    Draw3(DrawObj3),
}

pub enum DrawObj2 { // TODO expose
    // TODO rectangle
    // TODO image
    // TODO text
}

pub enum DrawObj3 {
    // TODO rectangle
    // TODO image
    // TODO text
    // TODO mesh
}

pub struct Canvas2<'a> { // TODO we could ignore the stack index thing and switch to a Drop based model?
    target: &'a mut FrameContent,
    stack_len: usize,
}

pub struct Canvas3<'a> {
    target: &'a mut FrameContent,
    stack_len: usize,
}

impl FrameContent {
    pub fn new() -> Self {
        Default::default() 
    }

    pub fn canvas(&mut self) -> Canvas2 {
        Canvas2 {
            target: self,
            stack_len: 0,
        }
    }
}

impl<'a> Canvas2<'a> {
    pub fn reborrow(&mut self) -> Canvas2 {
        Canvas2 {
            target: self.target,
            stack_len: self.stack_len,
        }
    }

    fn push(&mut self, instr: DrawInstr) {
        self.target.instrs.push((self.stack_len, instr));
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        self.push(DrawInstr::PushModifier2(modifier.into()));
        self.stack_len += 1;
        self
    }

    pub fn translate<V: Into<Vec2<f32>>>(self, v: V) -> Self {
        self.modify(Transform2::translate(v))
    }

    pub fn scale<V: Into<Vec2<f32>>>(self, v: V) -> Self {
        self.modify(Transform2::scale(v))
    }

    pub fn rotate(self, f: f32) -> Self {
        self.modify(Transform2::rotate(f))
    }

    pub fn color<C: Into<Rgba<f32>>>(self, c: C) -> Self {
        self.modify(c.into())
    }

    pub fn min_x(self, f: f32) -> Self {
        self.modify(Clip2::min_x(f))
    }

    pub fn max_x(self, f: f32) -> Self {
        self.modify(Clip2::max_x(f))
    }

    pub fn min_y(self, f: f32) -> Self {
        self.modify(Clip2::min_y(f))
    }

    pub fn max_y(self, f: f32) -> Self {
        self.modify(Clip2::max_y(f))
    }

    // TODO draw helpers
    pub fn draw<I: Into<DrawObj2>>(mut self, obj: I) -> Self {
        self.push(DrawInstr::Draw2(obj.into()));
        self
    }


    // TODO 3d helpers
    pub fn begin_3d<I: Into<ViewProj>>(mut self, view_proj: I) -> Canvas3<'a> {
        self.push(DrawInstr::Begin3d(view_proj.into()));
        Canvas3 {
            target: self.target,
            stack_len: self.stack_len + 1,
        }
    }
}

impl<'a> Canvas3<'a> {
    pub fn reborrow(&mut self) -> Canvas3 {
        Canvas3 {
            target: self.target,
            stack_len: self.stack_len,
        }
    }

    fn push(&mut self, instr: DrawInstr) {
        self.target.instrs.push((self.stack_len, instr));
    }

    pub fn modify<I: Into<Modifier3>>(mut self, modifier: I) -> Self {
        self.push(DrawInstr::PushModifier3(modifier.into()));
        self.stack_len += 1;
        self
    }

    pub fn translate<V: Into<Vec3<f32>>>(self, v: V) -> Self {
        self.modify(Transform3::translate(v))
    }

    pub fn scale<V: Into<Vec3<f32>>>(self, v: V) -> Self {
        self.modify(Transform3::scale(v))
    }

    pub fn rotate<Q: Into<Quaternion<f32>>>(self, q: Q) -> Self {
        self.modify(Transform3::rotate(q))
    }

    pub fn color<C: Into<Rgba<f32>>>(self, c: C) -> Self {
        self.modify(c.into())
    }

    pub fn min_x(self, f: f32) -> Self {
        self.modify(Clip3::min_x(f))
    }

    pub fn max_x(self, f: f32) -> Self {
        self.modify(Clip3::max_x(f))
    }

    pub fn min_y(self, f: f32) -> Self {
        self.modify(Clip3::min_y(f))
    }

    pub fn max_y(self, f: f32) -> Self {
        self.modify(Clip3::max_y(f))
    }

    pub fn min_z(self, f: f32) -> Self {
        self.modify(Clip3::min_z(f))
    }

    pub fn max_z(self, f: f32) -> Self {
        self.modify(Clip3::max_z(f))
    }

    // TODO draw helpers
    pub fn draw<I: Into<DrawObj3>>(mut self, obj: I) -> Self {
        self.push(DrawInstr::Draw3(obj.into()));
        self
    }
}


// ==== draw 3d normalization


pub enum DrawInstr3dNorm<O> {
    PushModifier {
        modifier: Modifier3,
        is_begin_3d: bool,
    },
    Draw(O),
}

impl<O> DrawInstr3dNorm<O> {
    pub fn map_obj<F, O2>(self, f: F) -> DrawInstr3dNorm<O2>
    where
        F: FnOnce(O) -> O2,
    {
        match self {
            DrawInstr3dNorm::PushModifier {
                modifier,
                is_begin_3d,
            } => DrawInstr3dNorm::PushModifier {
                modifier,
                is_begin_3d,
            },
            DrawInstr3dNorm::Draw(o) => DrawInstr3dNorm::Draw(f(o)),
        }
    }
}

pub enum DrawObj3dNorm {
    // TODO
}

impl From<&DrawObj2> for DrawObj3dNorm {
    fn from(obj: &DrawObj2) -> Self {
        match obj {
            _ => todo!(),
        }
    }
}

impl From<&DrawObj3> for DrawObj3dNorm {
    fn from(obj: &DrawObj3) -> Self {
        match obj {
            _ => todo!(),
        }
    }
}

pub struct Draw3dNormalizer<I> {
    inner: I,
}

impl<I> Draw3dNormalizer<I> {
    pub fn new(inner: I) -> Self {
        Draw3dNormalizer { inner }
    }
}

impl<'a, I> Iterator for Draw3dNormalizer<I>
where
    I: Iterator<Item=(usize, &'a DrawInstr)>,
{
    type Item = (usize, DrawInstr3dNorm<DrawObj3dNorm>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(stack_len, instr)| (stack_len, match instr {
                &DrawInstr::PushModifier2(m) => DrawInstr3dNorm::PushModifier {
                    modifier: m.to_3d(),
                    is_begin_3d: false,
                },
                &DrawInstr::Draw2(ref o) => DrawInstr3dNorm::Draw(o.into()),
                &DrawInstr::Begin3d(vp) => {
                    // TODO additional coordinate system conversion matrices?
                    DrawInstr3dNorm::PushModifier {
                        modifier: Transform3(vp.0).into(),
                        is_begin_3d: true,
                    }
                },
                &DrawInstr::PushModifier3(m) => DrawInstr3dNorm::PushModifier {
                    modifier: m,
                    is_begin_3d: false,
                },
                &DrawInstr::Draw3(ref o) => DrawInstr3dNorm::Draw(o.into()),
            }))
    }
}

// ==== draw impl compilation 


pub enum DrawImplInstr<O> {
    /// Draw a draw object. Always test against clip buffers.
    Draw {
        /// The draw object to draw.
        obj: O,
        /// Matrix with which to affine-transform the object.
        transform: Mat4<f32>,
        /// Color by which to multiply the object.
        color: Rgba<u8>,
        /// Whether to test against and write to the depth buffer.
        depth: bool,
    },
    /// Clear the clip min and clip max buffers.
    ClearClip,
    /// Set each element in the clip min buffer to the max of itself and
    /// -(ax+by+d)/c, wherein the contained vector is considered <a,b,c,d>
    /// and the coordinates of the buffer element are considered <x,y>.
    ///
    /// c may equal 0, in which case, produce positive/negative infinity as
    /// appropriate.
    MaxClipMin(Vec4<f32>),
    /// Like `MaxClipMin`, except set each element in the clip _max_ buffer to
    /// the _min_ of itself and the computed value.
    MinClipMax(Vec4<f32>),
    /// Clear the depth buffer to 1.
    ClearDepth,
}

pub struct DrawImplCompiler<I, O> {
    inner: I,
    trying_to_draw: Option<O>,
    stack: Vec<StackEntry>,
    cumul_transform_stack: Vec<Transform3>,
    cumul_color_stack: Vec<Rgba<f32>>,
    clip_stack: Vec<Clip3>,
    currently_3d: bool,
    clip_valid_up_to: Option<usize>,
    depth_valid: bool,
}

struct StackEntry {
    kind: ModifierKind,
    is_begin_3d: bool,
}

enum ModifierKind {
    Transform,
    Color,
    Clip,
}

impl<I, O> DrawImplCompiler<I, O> {
    pub fn new(inner: I) -> Self {
        // TODO coordinate system conversion?
        let base_transform = Transform3::identity();
        let base_color = Rgba::white();

        DrawImplCompiler {
            inner,
            trying_to_draw: None,
            stack: Vec::new(),
            cumul_transform_stack: vec![base_transform],
            cumul_color_stack: vec![base_color],
            clip_stack: Vec::new(),
            currently_3d: false,
            clip_valid_up_to: None,
            depth_valid: false,
        }
    }

    pub fn transform(&self) -> Transform3 {
        *self.cumul_transform_stack.last().unwrap()
    }

    pub fn color(&self) -> Rgba<f32> {
        *self.cumul_color_stack.last().unwrap()
    }
}

impl<I, O> Iterator for DrawImplCompiler<I, O>
where
    I: Iterator<Item=(usize, DrawInstr3dNorm<O>)>,
{
    type Item = DrawImplInstr<O>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(loop {
            if self.trying_to_draw.is_some() {
                break if self.clip_valid_up_to.is_none() {
                    self.clip_valid_up_to = Some(0);
                    DrawImplInstr::ClearClip
                } else if self.clip_valid_up_to.unwrap() < self.clip_stack.len() {
                    let clip = self.clip_stack[self.clip_valid_up_to.unwrap()];
                    *self.clip_valid_up_to.as_mut().unwrap() += 1;
                    if clip.0.z > 0.0 {
                        DrawImplInstr::MaxClipMin(clip.0)
                    } else {
                        DrawImplInstr::MinClipMax(clip.0)
                    }
                } else if self.currently_3d && !self.depth_valid {
                    self.depth_valid = true;
                    DrawImplInstr::ClearDepth
                } else {
                    let obj = self.trying_to_draw.take().unwrap();
                    let color = self.color()
                        .map(|n| (n.max(0.0).min(1.0) * 255.0) as u8);
                    DrawImplInstr::Draw {
                        obj,
                        transform: self.transform().0,
                        color,
                        depth: self.currently_3d,
                    }
                }
            } else {
                let (stack_len, instr) = self.inner.next()?;

                while self.stack.len() > stack_len {
                    let entry = self.stack.pop().unwrap();
                    match entry.kind {
                        ModifierKind::Transform => {
                            self.cumul_transform_stack.pop().unwrap();
                        }
                        ModifierKind::Color => {
                            self.cumul_color_stack.pop().unwrap();
                        }
                        ModifierKind::Clip => {
                            self.clip_stack.pop().unwrap();
                            if let Some(i) = self.clip_valid_up_to {
                                if i > self.clip_stack.len() {
                                    self.clip_valid_up_to = None;
                                }
                            }
                        }
                    }
                    if entry.is_begin_3d {
                        debug_assert!(self.currently_3d);
                        self.currently_3d = false;
                        self.depth_valid = false;
                    }
                }

                match instr {
                    DrawInstr3dNorm::PushModifier {
                        modifier,
                        is_begin_3d,
                    } => {
                        let kind = match modifier {
                            Modifier3::Transform(t) => {
                                self.cumul_transform_stack.push(t.then(&self.transform()));
                                ModifierKind::Transform
                            }
                            Modifier3::Color(c) => {
                                self.cumul_color_stack.push(c * self.color());
                                ModifierKind::Color
                            }
                            Modifier3::Clip(c) => {
                                self.clip_stack.push(c);
                                ModifierKind::Clip
                            }
                        };
                        self.stack.push(StackEntry {
                            kind,
                            is_begin_3d,
                        });
                        if is_begin_3d {
                            debug_assert!(!self.currently_3d);
                            self.currently_3d = true;
                        }
                    }
                    DrawInstr3dNorm::Draw(obj) => {
                        self.trying_to_draw = Some(obj);
                    }
                }
            }
        })
    }
}
