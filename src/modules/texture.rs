use anyhow::{anyhow, Result};
use glium::{backend::Context, texture::{RawImage2d, SrgbTexture2d}};
use std::rc::Rc;

pub struct Texture {
    pub data: SrgbTexture2d,
}

impl Texture {
    /// Creates a texture from raw RGBA bytes.
    pub fn from_rgba(bytes: &[u8], dimensions: (u32, u32), ctx: &Rc<Context>) -> Result<Self> {
        Ok(Self {
            data: SrgbTexture2d::new(ctx, RawImage2d::from_raw_rgba_reversed(bytes, dimensions))?,
        })
    }

    /// Loads `assets/textures/{path}` into an sRGB texture.
    pub fn from(path: &str, ctx: &Rc<Context>) -> Result<Self> {
        let full = format!("assets/textures/{path}");
        let img  = image::open(&full)
            .map_err(|_| anyhow!("Failed to open texture: {full}"))?
            .to_rgba8();
        Self::from_rgba(img.as_raw(), img.dimensions(), ctx)
    }
}
