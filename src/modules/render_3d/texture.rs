use anyhow::{anyhow, Result};
use glium::{backend::Context, texture::{RawImage2d, SrgbTexture2d}};
use std::rc::Rc;

pub struct Texture {
    pub data: SrgbTexture2d,
}

impl Texture {
    /// Raw texture
    pub fn from_rgba(bytes: &[u8], dimensions: (u32, u32), cont: &Rc<Context>) -> Result<Self> {
        Ok(Self {
            data: SrgbTexture2d::new(cont, RawImage2d::from_raw_rgba_reversed(bytes, dimensions))?,
        })
    }

    /// Loads a file into RGBA
    pub fn from(path: &str, cont: &Rc<Context>) -> Result<Self> {
        let path = format!("assets/textures/{path}");
        let img  = image::open(&path)
            .map_err(|_| anyhow!("Failed to open texture: {path}"))?
            .to_rgba8();
        Self::from_rgba(img.as_raw(), img.dimensions(), cont)
    }
}
