//! Transformation logic for 2D canvases.


use crate::std140::std140_struct;
use vek::*;


/// Accumulated transforms on a `Canvas2d`.
///
/// TODO this is not correct
#[derive(Debug, Copy, Clone)]
pub struct Canvas2dTransform {
    affine: Mat3<f32>,
    color: Rgba<f32>,

    clip_min_x: Option<f32>,
    clip_max_x: Option<f32>,
    clip_min_y: Option<f32>,
    clip_max_y: Option<f32>,
}

impl Canvas2dTransform {
    /// Identity transform.
    pub fn identity() -> Self {
        Canvas2dTransform {
            affine: Mat3::identity(),
            color: Rgba::white(),
            clip_min_x: None,
            clip_max_x: None,
            clip_min_y: None,
            clip_max_y: None,
        }
    }

    /// Apply translation.
    pub fn with_translate(self, t: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::translation_2d(t),
            ..self
        }
    }

    /// Apply scaling.
    ///
    /// Assumes no negative scaling, that would break the clipping logic.
    pub fn with_scale(self, s: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::scaling_3d([s.x, s.y, 1.0]),
            ..self
        }
    }

    /// Apply color multiplication.
    pub fn with_color(self, c: Rgba<f32>) -> Self {
        Canvas2dTransform {
            color: self.color * c,
            ..self
        }
    }

    /// Apply min-x clipping.
    pub fn with_clip_min_x(self, min_x: f32) -> Self {
        let min_x = (self.affine * Vec3::new(min_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_min_x: Some(self.clip_min_x
                .map(|x| f32::max(x, min_x))
                .unwrap_or(min_x)),
            ..self
        }
    }

    /// Apply max-x clipping.
    pub fn with_clip_max_x(self, max_x: f32) -> Self {
        let max_x = (self.affine * Vec3::new(max_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_max_x: Some(self.clip_max_x
                .map(|x| f32::min(x, max_x))
                .unwrap_or(max_x)),
            ..self
        }
    }

    /// Apply min-y clipping.
    pub fn with_clip_min_y(self, min_y: f32) -> Self {
        let min_y = (self.affine * Vec3::new(0.0, min_y, 1.0)).y;
        Canvas2dTransform {
            clip_min_y: Some(self.clip_min_y
                .map(|x| f32::max(x, min_y))
                .unwrap_or(min_y)),
            ..self
        }
    }

    /// Apply max-y clipping.
    pub fn with_clip_max_y(self, max_y: f32) -> Self {
        let max_y = (self.affine * Vec3::new(0.0, max_y, 1.0)).y;
        Canvas2dTransform {
            clip_max_y: Some(self.clip_max_y
                .map(|x| f32::min(x, max_y))
                .unwrap_or(max_y)),
            ..self
        }
    }

    /// Convert to uniform data.
    pub fn to_uniform_data(&self) -> Canvas2dUniformData {
        Canvas2dUniformData {
            transform: self.affine,
            color: self.color,
            clip_min_x: self.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.clip_max_y.unwrap_or(f32::INFINITY),
        }
    }
}


/// Data for the canvas2d uniform buffer, which holds its transform
/// information.
#[derive(Debug, Copy, Clone)]
pub struct Canvas2dUniformData {
    pub transform: Mat3<f32>,
    pub color: Rgba<f32>,
    pub clip_min_x: f32,
    pub clip_max_x: f32,
    pub clip_min_y: f32,
    pub clip_max_y: f32,
}

std140_struct! {
    Canvas2dUniformData {
        transform: Mat3<f32>,
        color: Rgba<f32>,
        clip_min_x: f32,
        clip_max_x: f32,
        clip_min_y: f32,
        clip_max_y: f32,
    }
}
