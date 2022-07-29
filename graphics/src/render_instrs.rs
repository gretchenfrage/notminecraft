
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
    },
};
use vek::*;


// ==== frame item normalization ====

#[derive(Debug, Clone)]
enum FrameItemNorm {
    PushModifier {
        modifier: Modifier3,
        is_begin_3d: bool,
    },
    Draw(DrawObjNorm),
}

#[derive(Debug, Clone)]
pub enum DrawObjNorm {
    Solid,
    // TODO
}

impl From<&DrawObj2> for DrawObjNorm {
    fn from(obj: &DrawObj2) -> Self {
        match obj {
            &DrawObj2::Solid => DrawObjNorm::Solid,
        }
    }
}

impl From<&DrawObj3> for DrawObjNorm {
    fn from(obj: &DrawObj3) -> Self {
        match obj {
            &DrawObj3::Solid => DrawObjNorm::Solid,
        }
    }
}

#[derive(Debug, Clone)]
struct Normalize<I>(I);

impl<'a, I> Iterator for Normalize<I>
where
    I: Iterator<Item=(usize, &'a FrameItem)>,
{
    type Item = (usize, FrameItemNorm);

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
pub enum RenderInstr {
    /// Draw a draw object. Always test against clip buffers.
    Draw {
        /// The draw object to draw.
        obj: DrawObjNorm,
        /// Matrix with which to affine-transform the object.
        transform: Mat4<f32>,
        /// Color by which to multiply the object.
        color: Rgba<u8>,
        /// Whether to test against and write to the depth buffer.
        depth: bool,
    },
    /// Clear the clip min and clip max buffers.
    ClearClip,
    /*
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
    */
    /*
    EditClip {

        affine: Vec3<f32>,
        max_clip_min: bool,
    }*/
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
            max_clip_min: clip.0.z > 0.0,
            clip: clip.0,
        }
        /*
        // -(ax+by+d)/c == <i,j,k> dot <x,y,1>
        // if c == 0 should be some infinity
        // c == 0 -> <i,j,k> == <0,0,

        let [a, b, c, d] = dbg!(clip.0.into_array());
        let max_clip_min = c > 0.0;
        let affine = dbg!(Vec3 {
            x: -a / c,
            y: -b / c,
            z: -d / c,
        });
        // TODO: double check later that infinities work correctly
        ClipEdit { max_clip_min, affine }*/
    }
}

#[derive(Debug, Clone)]
pub struct RenderCompiler<I> {
    inner: Normalize<I>,
    trying_to_draw: Option<DrawObjNorm>,
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

impl<I> RenderCompiler<I> {
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

impl<'a, I> Iterator for RenderCompiler<I>
where
    I: Iterator<Item=(usize, &'a FrameItem)>,
{
    type Item = RenderInstr;

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
