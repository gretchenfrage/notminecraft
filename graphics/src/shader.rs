//! Loading shaders.
//!
//! Shaders are stored in `src/shaders`. With the `shaderc` feature enabled,
//! this will link to shaderc, and load and compile the glsl files from the
//! source tree at run time. Without the `shaderc` feature enabled, this
//! expected `.spv` files to be pre-compiled, and will bake them into the
//! compiled binary.

pub use mod_impl::*;


/// Load and compile a shader with the given name.
/// 
/// Behaves like an `async fn(&str) -> Result<ShaderModuleDescriptor<'static>>`,
/// although the input must be a literal.
pub(crate) use mod_impl::load_shader;


#[cfg(feature = "shaderc")]
#[doc(hidden)]
pub mod mod_impl {
    use std::{
        path::Path,
        fs,
    };
    use anyhow::Result;
    use wgpu::*;
    use shaderc::{
        Compiler,
        ShaderKind,
    };

    macro_rules! load_shader {
        ($name:literal)=>{
            $crate::shader::mod_impl::load_shader_impl($name)
        };
    }

    pub(crate) use load_shader;

    pub fn load_shader_impl(name: &'static str) -> Result<ShaderModuleDescriptor<'static>> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders").join(name);
        let glsl = fs::read(&path)?;
        let glsl = String::from_utf8(glsl)
            .map_err(|_| anyhow::Error::msg("shader not utf-8"))?;

        let kind =
            if name.ends_with(".vert") { ShaderKind::Vertex }
            else if name.ends_with(".frag") { ShaderKind::Fragment }
            else { return Err(anyhow::Error::msg("unknown shader kind")) };

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
}

#[cfg(not(feature = "shaderc"))]
#[doc(hidden)]
pub mod mod_impl {
    use anyhow::Result;
    use wgpu::{
        *,
        util::make_spirv_raw,
    };

    macro_rules! load_shader {
        ($name:literal)=>{
            $crate::shader::mod_impl::load_shader_impl(
                $name,
                ::core::include_bytes!(
                    ::core::concat!(
                        ::core::env!("CARGO_MANIFEST_DIR"),
                        "/src/shaders/",
                        $name,
                        ".spv"
                    )
                )
            )
        };
    }

    pub(crate) use load_shader;

    pub fn load_shader_impl(
        name: &'static str,
        data: &'static [u8],
    ) -> Result<ShaderModuleDescriptor<'static>> {
        Ok(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::SpirV(make_spirv_raw(data)),
        })
    }
}
