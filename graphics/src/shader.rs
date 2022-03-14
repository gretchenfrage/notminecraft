//! Loading shaders.

use std::path::Path;
use anyhow::Result;
use tokio::fs;
use wgpu::*;
use shaderc::{
    Compiler,
    ShaderKind,
};


/// Load and compile a shader with the given name.
pub async fn load_shader(name: &'static str) -> Result<ShaderModuleDescriptor<'static>> {
    let path = Path::new("src/shaders").join(name);
    let glsl = fs::read(&path).await?;
    let glsl = String::from_utf8(glsl)
        .map_err(|_| anyhow::Error::msg("shader not utf-8"))?;

    let kind =
        if name.ends_with(".vert") { ShaderKind::Vertex }
        else if name.ends_with(".frag") { ShaderKind::Fragment }
        else { return Err(anyhow::Error::msg("unknown chader kind")) };

    let mut compiler = Compiler::new()
        .ok_or_else(|| anyhow::Error::msg("no shaderc compiler"))?;

    let artifact = compiler.compile_into_spirv(
        &glsl,
        kind,
        name,
        "main",
        None,
    )?;

    Ok(ShaderModuleDescriptor {
        label: Some(name),
        source: ShaderSource::SpirV(artifact.as_binary().to_owned().into()),
    })
}
