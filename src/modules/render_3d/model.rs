use asset_importer::{Scene, TextureData, TextureType::Diffuse, Vector2D};
use anyhow::{anyhow, Result};
use glium::{
    DrawParameters, Frame, IndexBuffer, Program, Surface, VertexBuffer, backend::Context, implement_vertex, index::{Index, PrimitiveType}, uniform
};
use image::ImageBuffer;
use rclite::Rc;
use rustc_hash::FxHashMap;
use core::f32;
use std::{ops::Range, rc::Rc as StdRc};
use crate::modules::{render_3d::{aabb::Aabb, camera::Camera, transform::Transform}, texture::Texture};

// ── Vertex ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv:       [f32; 2],
}
implement_vertex!(Vertex, position, uv);

// ── MMaterial / SubMesh ────────────────────────────────────────────────────────
#[derive(Clone)]
pub struct MMaterial { pub tex: Rc<Texture> }

#[derive(Clone)]
pub struct SubMesh {
    pub range:    Range<usize>,
    pub material: usize,
}

impl SubMesh {
    pub fn new(range: Range<usize>, material: usize) -> Self {
        Self { range, material }
    }
}

pub enum MCacheP {
    Shared(rclite::Rc<ModelCache>),
    Unique(ModelCache),
}

// ── Model ─────────────────────────────────────────────────────────────────────
// Maybe store only the id(usize) from the FxHashMap, has to request the cache first
/// Literally a wrapper to separate unique data
pub struct Model {
    pub transform: Transform,
    pub shader_p:  Rc<Program>,
    pub cache:     MCacheP,
}

impl Model {
    #[allow(unused)]
    pub fn set_position(&mut self, x: f32, y: f32, z: f32) {
        self.transform.set_position(x, y, z);
    }

    pub fn draw(
        &self,
        frame: &mut Frame,
        cam: &Camera,
        params: &DrawParameters<'_>,
    ) -> Result<()> {
        match &self.cache {
            MCacheP::Shared(x) => x.draw(&self.shader_p, frame, cam, &self.transform, params)?,
            MCacheP::Unique(x) => x.draw(&self.shader_p, frame, cam, &self.transform, params)?,
        }
        Ok(())
    }
}

// ── Buffers ───────────────────────────────────────────────────────────────────

pub fn new_buffers<V, I>(
    verts: &[V],
    inds: &[I],
    ctx: &StdRc<Context>,
    dynamic: bool,
) -> Result<(VertexBuffer<V>, IndexBuffer<I>)>
where
    V: glium::Vertex + Copy,
    I: Index + Copy,
{
    Ok((
        if dynamic { VertexBuffer::dynamic(ctx, verts) }
        else { VertexBuffer::new(ctx, verts) }
            .map_err(|_| anyhow!("Failed to create vertex buffer"))?,
        IndexBuffer::new(ctx, PrimitiveType::TrianglesList, inds)
            .map_err(|_| anyhow!("Failed to create index buffer"))?,
    ))
}

// ── ModelCache ────────────────────────────────────────────────────────────────
pub struct ModelCache {
    pub v_buffer:   VertexBuffer<Vertex>,
    pub i_buffer:   IndexBuffer<u32>,
    pub sub_meshes: Vec<SubMesh>,
    pub materials:  Vec<MMaterial>,
    pub bounds:     Aabb,
}

impl ModelCache {
    /// Loads: Vertices, Faces, Textures, UVs. As sub-meshes.
    pub fn load(
        path: &str,
        ctx: &StdRc<Context>,
        default_tex: &Rc<Texture>,
        dynamic: bool,
        texture_cache: &mut FxHashMap<String, rclite::Rc<Texture>>
    ) -> Result<Self> {
        let scene = Scene::from_file(path).map_err(|_| anyhow!("Failed to load scene: {path}"))?;

        let mut materials     = Vec::with_capacity(scene.num_materials());
        for mat in scene.materials() {
            let mut m= MMaterial { tex: Rc::clone(default_tex) };
            if let Some(t) = mat.texture(Diffuse, 0) {
                let path = &t.path;
                if !texture_cache.contains_key(path) {
                    let tex = Texture::from(path, &ctx)?;
                    texture_cache.insert(path.to_string(), Rc::new(tex));
                }
                m.tex = Rc::clone(&texture_cache[path]);
            }
            materials.push(m);
        }

        let mut vertices:   Vec<Vertex>  = vec![];
        let mut indices:    Vec<u32>     = vec![];
        let mut sub_meshes: Vec<SubMesh> = vec![];
        let mut v_offset:   u32          = 0;
        for mesh in scene.meshes() {
            let verts = mesh.vertices();
            let uvs = match mesh.texture_coords2(0) {
                Some(c) if c.len() == verts.len() => c,
                _ => vec![Vector2D { x: 0.0, y: 0.0 }; verts.len()],
            };
            let start = indices.len();
            mesh.faces().for_each(|f| indices.extend(f.indices().iter().map(|&i| i + v_offset)));
            sub_meshes.push(SubMesh::new(start..indices.len(), mesh.material_index()));
            vertices.extend(uvs.iter().zip(verts.iter()).map(|(t, v)| Vertex {
                position: [v.x, v.y, v.z],
                uv:       [t.x, t.y],
            }));
            v_offset += verts.len() as u32;
        }

        let bounds = Aabb::from_positions(vertices.iter().map(|v| v.position));

        // Embedded textures (stored but not yet bound to materials)
        let _embedded: Vec<_> = if scene.has_textures() {
            scene.textures().filter_map(|tex| {
                let (w, h) = tex.dimensions();
                tex.data().ok().and_then(|data| match data {
                    TextureData::Compressed(b) =>
                        image::load_from_memory(&b).ok().map(|i| i.to_rgba8()),
                    TextureData::Texels(t) => {
                        let raw: Vec<u8> = t.into_iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
                        ImageBuffer::from_raw(w, h, raw)
                    }
                })
            }).collect()
        } else {
            vec![]
        };

        let (v_buffer, i_buffer) = new_buffers(&vertices, &indices, ctx, dynamic)?;

        Ok(Self {
            v_buffer,
            i_buffer,
            sub_meshes,
            materials,
            bounds,
        })
    }

    // Optional:
    //  Instancing: Repeat model in n positions.
    //  Batching:   Buffering across small models.
    pub fn draw(
        &self,
        shader:     &Program,
        frame:      &mut Frame,
        cam:        &Camera,
        transform:  &Transform,
        params:     &DrawParameters<'_>,
    ) -> Result<()> {
        if !transform.frustum_cull(&cam.planes, &self.bounds) { return Ok(()) }

        let v_buffer = &self.v_buffer;
        let i_buffer = &self.i_buffer;
        let materials = &self.materials;
        let matrix = transform.matrix_array;
        for sub in &self.sub_meshes {
            frame.draw(
                v_buffer,
                i_buffer.slice(sub.range.clone())
                    .ok_or_else(|| anyhow!("Invalid index buffer range: {:?}", sub.range))?,
                shader,
                &uniform! {
                    matrix:    matrix,
                    vp:        cam.arr_vp,
                    texture2D: materials[sub.material].tex.data.sampled()
                        // Repeat with UV
                        .wrap_function(glium::uniforms::SamplerWrapFunction::Repeat)
                        // Pixels to nearest
                        .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                        .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
                },
                params,
            )?;
        }
        Ok(())
    }
}