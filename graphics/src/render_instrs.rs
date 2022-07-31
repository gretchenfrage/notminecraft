
use crate::{
    modifier::{
        Modifier3,
        Transform3,
        Clip3,
        Transform2,
    },
    frame_content::{
        FrameContent,
        FrameItem,
        DrawObj2,
        DrawObj3,
        DrawImage,
    },
};
use vek::*;


// ==== frame item normalization ====

#[derive(Debug, Clone)]
enum FrameItemNorm<'a> {
    PushModifier {
        modifier: Modifier3,
        is_begin_3d: bool,
    },
    Draw(DrawObjNorm<'a>),
}

#[derive(Debug, Clone)]
pub enum DrawObjNorm<'a> {
    Solid,
    Image(&'a DrawImage),
}

impl<'a> From<&'a DrawObj2> for DrawObjNorm<'a> {
    fn from(obj: &'a DrawObj2) -> Self {
        match obj {
            &DrawObj2::Solid => DrawObjNorm::Solid,
            &DrawObj2::Image(ref obj) => DrawObjNorm::Image(obj),
        }
    }
}

impl<'a> From<&'a DrawObj3> for DrawObjNorm<'a> {
    fn from(obj: &'a DrawObj3) -> Self {
        match obj {
            &DrawObj3::Solid => DrawObjNorm::Solid,
            &DrawObj3::Image(ref obj) => DrawObjNorm::Image(obj),
        }
    }
}

#[derive(Debug, Clone)]
struct Normalize<I>(I);

impl<'a, I> Iterator for Normalize<I>
where
    I: Iterator<Item=(usize, &'a FrameItem)>,
{
    type Item = (usize, FrameItemNorm<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(stack_len, instr)| (
                stack_len,
                match instr {
                    &FrameItem::PushModifier2(m) => FrameItemNorm::PushModifier {
                        modifier: m.to_3d(),
                        is_begin_3d: false,
                    },
                    &FrameItem::Draw2(ref o) => FrameItemNorm::Draw(o.into()),
                    &FrameItem::Begin3d(vp) => {
                        // TODO additional coordinate system conversion matrices?
                        FrameItemNorm::PushModifier {
                            modifier: Transform3(vp.0).into(),
                            is_begin_3d: true,
                        }
                    },
                    &FrameItem::PushModifier3(m) => FrameItemNorm::PushModifier {
                        modifier: m,
                        is_begin_3d: false,
                    },
                    &FrameItem::Draw3(ref o) => FrameItemNorm::Draw(o.into()),
                },
            ))
    }
}



// ==== instruction compiler ====

pub fn frame_render_compiler<'a>(
    content: &'a FrameContent
) -> RenderCompiler<impl Iterator<Item=(usize, &'a FrameItem)> + 'a>
{
    let items = content.0
        .iter()
        .map(|&(stack_len, ref item)| (stack_len, item));
    RenderCompiler::new(items)
}


#[derive(Debug, Clone)]
pub enum RenderInstr<'a> {
    /// Draw a draw object. Always test against clip buffers.
    Draw {
        /// The draw object to draw.
        obj: DrawObjNorm<'a>,
        /// Matrix with which to affine-transform the object.
        transform: Mat4<f32>,
        /// Color by which to multiply the object.
        color: Rgba<u8>,
        /// Whether to test against and write to the depth buffer.
        depth: bool,
    },
    /// Clear the clip min and clip max buffers.
    ClearClip,
    EditClip(ClipEdit),
    /// Clear the depth buffer to 1.
    ClearDepth,
}

#[derive(Debug, Copy, Clone)]
pub struct ClipEdit {
    pub max_clip_min: bool,
    pub clip: Vec4<f32>,
}

impl From<Clip3> for ClipEdit {
    fn from(clip: Clip3) -> Self {
        ClipEdit {
            max_clip_min: clip.0.z >= 0.0,
            clip: clip.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderCompiler<'a, I> {
    inner: Normalize<I>,
    trying_to_draw: Option<DrawObjNorm<'a>>,
    stack: Vec<StackEntry>,
    cumul_transform_stack: Vec<Transform3>,
    cumul_color_stack: Vec<Rgba<f32>>,
    clip_stack: Vec<Clip3>,
    currently_3d: bool,
    clip_valid_up_to: Option<usize>,
    depth_valid: bool,
}

#[derive(Debug, Clone)]
struct StackEntry {
    kind: ModifierKind,
    is_begin_3d: bool,
}

#[derive(Debug, Clone)]
enum ModifierKind {
    Transform,
    Color,
    Clip,
}

impl<'a, I> RenderCompiler<'a, I> {
    pub fn new(inner: I) -> Self {
        // TODO coordinate system conversion?
        //let base_transform = Transform3::identity();
        let base_transform = Transform2(Mat3::new(
            2.0, 0.0, -1.0,
            0.0, -2.0, 1.0,
            0.0, 0.0, 1.0, 
        )).to_3d();
        let base_color = Rgba::white();

        RenderCompiler {
            inner: Normalize(inner),
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

impl<'a, I> Iterator for RenderCompiler<'a, I>
where
    I: Iterator<Item=(usize, &'a FrameItem)>,
{
    type Item = RenderInstr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(loop {
            if self.trying_to_draw.is_some() {
                break if self.clip_valid_up_to.is_none() {
                    self.clip_valid_up_to = Some(0);
                    RenderInstr::ClearClip
                } else if self.clip_valid_up_to.unwrap() < self.clip_stack.len() {
                    let clip = self.clip_stack[self.clip_valid_up_to.unwrap()];
                    *self.clip_valid_up_to.as_mut().unwrap() += 1;
                    RenderInstr::EditClip(ClipEdit::from(clip))
                } else if self.currently_3d && !self.depth_valid {
                    self.depth_valid = true;
                    RenderInstr::ClearDepth
                } else {
                    let obj = self.trying_to_draw.take().unwrap();
                    let color = self.color()
                        .map(|n| (n.max(0.0).min(1.0) * 255.0) as u8);
                    RenderInstr::Draw {
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
                    FrameItemNorm::PushModifier {
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
                                let c = self.transform().apply_clip(&c);
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
                    FrameItemNorm::Draw(obj) => {
                        self.trying_to_draw = Some(obj);
                    }
                }
            }
        })
    }
}
