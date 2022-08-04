
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
use vek::*;


pub use crate::pipelines::{
    image::{
        GpuImage,
        DrawImage,
    },
    text::{
        TextBlock,
        TextSpan,
        HorizontalAlign,
        VerticalAlign,
        LayedOutTextBlock,
        FontId,
    },
    mesh::{
        GpuVec,
        GpuVecElem,
        GpuImageArray,
        Mesh,
        Vertex,
        Triangle,
        DrawMesh,
    },
};


#[derive(Debug, Clone, Default)]
pub struct FrameContent<'a>(pub Vec<(usize, FrameItem<'a>)>);

#[derive(Debug, Clone)]
pub enum FrameItem<'a> {
    PushModifier2(Modifier2),
    Draw2(DrawObj2),
    Begin3d(ViewProj),
    PushModifier3(Modifier3),
    Draw3(DrawObj3<'a>),
}

#[derive(Debug, Clone)]
pub enum DrawObj2 {
    Solid,
    Image(DrawImage),
    Text(LayedOutTextBlock),
}

#[derive(Debug, Clone)]
pub enum DrawObj3<'a> {
    Solid,
    Image(DrawImage),
    Text(LayedOutTextBlock),
    Mesh(DrawMesh<'a>),
}

#[derive(Debug)]
pub struct Canvas2<'a, 'b> {
    target: &'b mut FrameContent<'a>,
    stack_len: usize,
}

#[derive(Debug)]
pub struct Canvas3<'a, 'b> {
    target: &'b mut FrameContent<'a>,
    stack_len: usize,
}

impl<'a> FrameContent<'a> {
    pub fn new() -> Self {
        Default::default() 
    }

    pub fn canvas<'b>(&'b mut self) -> Canvas2<'a, 'b> {
        Canvas2 {
            target: self,
            stack_len: 0,
        }
    }
}

impl<'a, 'b> Canvas2<'a, 'b> {
    pub fn reborrow<'b2>(&'b2 mut self) -> Canvas2<'a, 'b2> {
        Canvas2 {
            target: self.target,
            stack_len: self.stack_len,
        }
    }

    fn push(&mut self, item: FrameItem<'a>) {
        self.target.0.push((self.stack_len, item));
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        self.push(FrameItem::PushModifier2(modifier.into()));
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
        self.push(FrameItem::Draw2(obj.into()));
        self
    }

    pub fn draw_solid<V: Into<Extent2<f32>>>(mut self, size: V) -> Self {
        self
            .reborrow()
            .scale(size.into())
            .draw(DrawObj2::Solid);
        self
    }

    pub fn draw_image<V: Into<Extent2<f32>>>(
        self,
        image: &GpuImage,
        size: V,
    ) -> Self
    {
        self
            .draw_image_uv(
                image,
                size,
                [0.0, 0.0],
                [1.0, 1.0],
            )
    }

    pub fn draw_image_uv<V1, V2, V3>(
        mut self,
        image: &GpuImage,
        size: V1,
        tex_start: V2,
        tex_extent: V3,
    ) -> Self
    where
        V1: Into<Extent2<f32>>,
        V2: Into<Vec2<f32>>,
        V3: Into<Extent2<f32>>,
    {
        self
            .reborrow()
            .scale(size.into())
            .draw(DrawObj2::Image(DrawImage {
                image: image.clone(),
                tex_start: tex_start.into(),
                tex_extent: tex_extent.into(),
            }));
        self
    }

    pub fn draw_text(self, text: &LayedOutTextBlock) -> Self
    {
        self.draw(DrawObj2::Text(text.clone()))
    }

    // TODO 3d helpers
    pub fn begin_3d<I: Into<ViewProj>>(mut self, view_proj: I) -> Canvas3<'a, 'b> {
        self.push(FrameItem::Begin3d(view_proj.into()));
        Canvas3 {
            target: self.target,
            stack_len: self.stack_len + 1,
        }
    }

    pub fn begin_3d_perspective(
        self,
        size: impl Into<Extent2<f32>>,
        pos: impl Into<Vec3<f32>>,
        dir: impl Into<Quaternion<f32>>,
        fov: f32,
    ) -> Canvas3<'a, 'b> {
        let size = size.into();
        self
            .scale(size)
            .begin_3d(ViewProj::perspective(
                pos,
                dir,
                fov,
                size.w / size.h,
            ))
    }
}

impl<'a, 'b> Canvas3<'a, 'b> {
    pub fn reborrow<'b2>(&'b2 mut self) -> Canvas3<'a, 'b2> {
        Canvas3 {
            target: self.target,
            stack_len: self.stack_len,
        }
    }

    fn push(&mut self, item: FrameItem<'a>) {
        self.target.0.push((self.stack_len, item));
    }

    pub fn modify<I: Into<Modifier3>>(mut self, modifier: I) -> Self {
        self.push(FrameItem::PushModifier3(modifier.into()));
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
    pub fn draw<I: Into<DrawObj3<'a>>>(mut self, obj: I) -> Self {
        self.push(FrameItem::Draw3(obj.into()));
        self
    }

    pub fn draw_solid(self) -> Self {
        self.draw(DrawObj3::Solid)
    }

    pub fn draw_image<V1: Into<Vec2<f32>>, V2: Into<Extent2<f32>>>(
        self,
        image: &GpuImage,
        tex_start: V1,
        tex_extent: V2,
    ) -> Self
    {
        self.draw(DrawObj3::Image(DrawImage {
            image: image.clone(),
            tex_start: tex_start.into(),
            tex_extent: tex_extent.into(),
        }))
    }

    pub fn draw_text(self, text: &LayedOutTextBlock) -> Self
    {
        self.draw(DrawObj3::Text(text.clone()))
    }

    pub fn draw_mesh(self, mesh: &'a Mesh, textures: &GpuImageArray) -> Self
    {
        self.draw(DrawObj3::Mesh(DrawMesh {
            mesh,
            textures: textures.clone(),
        }))
    }
}
