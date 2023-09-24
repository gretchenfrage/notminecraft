
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
        LayedOutTextBlock,
        DrawMesh,
        DrawInvert,
        DrawSky,
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
    PushDebugTag, // TODO merge in to push modifier?
}

#[derive(Debug, Clone)]
pub enum DrawObjNorm<'a> {
    Solid,
    Line,
    Image(&'a DrawImage),
    Text(&'a LayedOutTextBlock),
    Mesh(&'a DrawMesh<'a>),
    Invert(&'a DrawInvert),
    Sky(&'a DrawSky),
}

impl<'a> From<&'a DrawObj2> for DrawObjNorm<'a> {
    fn from(obj: &'a DrawObj2) -> Self {
        match obj {
            &DrawObj2::Solid => DrawObjNorm::Solid,
            &DrawObj2::Line => DrawObjNorm::Line,
            &DrawObj2::Image(ref obj) => DrawObjNorm::Image(obj),
            &DrawObj2::Text(ref obj) => DrawObjNorm::Text(obj),
            &DrawObj2::Invert(ref obj) => DrawObjNorm::Invert(obj),
            &DrawObj2::Sky(ref obj) => DrawObjNorm::Sky(obj),
        }
    }
}

impl<'a> From<&'a DrawObj3<'a>> for DrawObjNorm<'a> {
    fn from(obj: &'a DrawObj3) -> Self {
        match obj {
            &DrawObj3::Solid => DrawObjNorm::Solid,
            &DrawObj3::Line => DrawObjNorm::Line,
            &DrawObj3::Image(ref obj) => DrawObjNorm::Image(obj),
            &DrawObj3::Text(ref obj) => DrawObjNorm::Text(obj),
            &DrawObj3::Mesh(ref obj) => DrawObjNorm::Mesh(obj),
            &DrawObj3::Invert(ref obj) => DrawObjNorm::Invert(obj),
            &DrawObj3::Sky(ref obj) => DrawObjNorm::Sky(obj),
        }
    }
}

#[derive(Debug, Clone)]
struct Normalize<I>(I);

impl<'a, I> Iterator for Normalize<I>
where
    I: Iterator<Item=(usize, &'a FrameItem<'a>)>,
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
                    &FrameItem::PushDebugTag(_) => FrameItemNorm::PushDebugTag,
                },
            ))
    }
}



// ==== instruction compiler ====

pub fn frame_render_compiler<'a>(
    content: &'a FrameContent<'a>,
    surface_size: Extent2<u32>,
) -> RenderCompiler<impl Iterator<Item=(usize, &'a FrameItem<'a>)> + 'a>
{
    let items = content.0
        .iter()
        .map(|&(stack_len, ref item)| (stack_len, item));
    RenderCompiler::new(items, surface_size)
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
        /// Matrix which converts screenspace to worldspace positions.
        screen_to_world: Mat4<f32>,
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
    screen_to_world: Mat4<f32>,
    clip_stack: Vec<Clip3>,
    currently_3d: bool,
    clip_valid_up_to: Option<usize>,
    depth_valid: bool,
}

#[derive(Debug, Clone)]
struct StackEntry {
    kind: StackEntryKind,
    is_begin_3d: bool,
}

#[derive(Debug, Clone)]
enum StackEntryKind {
    Transform,
    Color,
    Clip,
    DebugTag,
}

fn base_transform(surface_size: Extent2<u32>) -> Transform3 {
    let convert = Transform2(Mat3::new(
        2.0, 0.0, -1.0,
        0.0, -2.0, 1.0,
        0.0, 0.0, 1.0, 
    ));
    let scale = Transform2::scale(surface_size.map(|n| 1.0 / n as f32));
    scale.then(&convert).to_3d()
}

impl<'a, I> RenderCompiler<'a, I> {
    pub fn new(inner: I, surface_size: Extent2<u32>) -> Self {
        let base_transform = base_transform(surface_size);
        let base_color = Rgba::white();

        RenderCompiler {
            inner: Normalize(inner),
            trying_to_draw: None,
            stack: Vec::new(),
            cumul_transform_stack: vec![base_transform],
            cumul_color_stack: vec![base_color],
            screen_to_world: Mat4::zero(),
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
    I: Iterator<Item=(usize, &'a FrameItem<'a>)>,
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
                    let screen_to_world = match &obj {
                        // doing this like this isn't architecturally consistent, buuutt
                        // it's BOTH more optimized and requires less code, so whatever
                        &DrawObjNorm::Sky(sky) => Transform3(sky.view_proj.0).then(&self.transform()).0.inverted(),
                        _ => self.screen_to_world,
                    };
                    RenderInstr::Draw {
                        obj,
                        transform: self.transform().0,
                        color,
                        screen_to_world,
                        depth: self.currently_3d,
                    }
                }
            } else {
                let (stack_len, instr) = self.inner.next()?;

                while self.stack.len() > stack_len {
                    let entry = self.stack.pop().unwrap();
                    if entry.is_begin_3d {
                        debug_assert!(self.currently_3d);
                        self.screen_to_world = Mat4::zero();
                        self.currently_3d = false;
                        self.depth_valid = false;
                    }
                    match entry.kind {
                        StackEntryKind::Transform => {
                            self.cumul_transform_stack.pop().unwrap();
                        }
                        StackEntryKind::Color => {
                            self.cumul_color_stack.pop().unwrap();
                        }
                        StackEntryKind::Clip => {
                            self.clip_stack.pop().unwrap();
                            if let Some(i) = self.clip_valid_up_to {
                                if i > self.clip_stack.len() {
                                    self.clip_valid_up_to = None;
                                }
                            }
                        }
                        StackEntryKind::DebugTag => (),
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
                                StackEntryKind::Transform
                            }
                            Modifier3::Color(c) => {
                                self.cumul_color_stack.push(c * self.color());
                                StackEntryKind::Color
                            }
                            Modifier3::Clip(c) => {
                                let c = self.transform().apply_clip(&c);
                                self.clip_stack.push(c);
                                StackEntryKind::Clip
                            }
                        };
                        self.stack.push(StackEntry {
                            kind,
                            is_begin_3d,
                        });
                        if is_begin_3d {
                            debug_assert!(!self.currently_3d);
                            self.currently_3d = true;
                            self.screen_to_world = self.transform().0.inverted();
                        }
                    }
                    FrameItemNorm::Draw(obj) => {
                        self.trying_to_draw = Some(obj);
                    }
                    FrameItemNorm::PushDebugTag => {
                        self.stack.push(StackEntry {
                            kind: StackEntryKind::DebugTag,
                            is_begin_3d: false,
                        })
                    }
                }
            }
        })
    }
}
