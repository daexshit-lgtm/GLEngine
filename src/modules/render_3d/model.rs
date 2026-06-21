use asset_importer::{Scene, TextureData, TextureType::Diffuse, Vector2D};
use anyhow::{anyhow, Result};
use glium::{
    DrawParameters, Frame, IndexBuffer, Program, Surface, VertexBuffer,
    backend::Context, implement_vertex, index::{Index, PrimitiveType}, uniform,
};
use image::ImageBuffer;
use rclite::Rc;
use rustc_hash::FxHashMap;
use std::{ops::Range, rc::Rc as StdRc};
use crate::modules::{
    render_3d::{aabb::Aabb, camera::Camera, transform::Transform},
    texture::Texture,
};

// ── Vertex ────────────────────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv:       [f32; 2],
}
implement_vertex!(Vertex, position, uv);

// ── MMaterial / SubMesh ───────────────────────────────────────────────────────
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
    Shared(Rc<ModelCache>),
    Unique(ModelCache),
}

// ── Model ─────────────────────────────────────────────────────────────────────
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

    pub fn draw(&self, frame: &mut Frame, cam: &Camera, params: &DrawParameters<'_>) -> Result<()> {
        match &self.cache {
            MCacheP::Shared(c) => c.draw(&self.shader_p, frame, cam, &self.transform, params),
            MCacheP::Unique(c) => c.draw(&self.shader_p, frame, cam, &self.transform, params),
        }
    }
}

// ── Buffer helpers ────────────────────────────────────────────────────────────
pub fn new_buffers<V, I>(
    verts:   &[V],
    inds:    &[I],
    ctx:     &StdRc<Context>,
    dynamic: bool,
) -> Result<(VertexBuffer<V>, IndexBuffer<I>)>
where
    V: glium::Vertex + Copy,
    I: Index + Copy,
{
    let vb = if dynamic { VertexBuffer::dynamic(ctx, verts) } else { VertexBuffer::new(ctx, verts) }
        .map_err(|_| anyhow!("Failed to create vertex buffer"))?;
    let ib = IndexBuffer::new(ctx, PrimitiveType::TrianglesList, inds)
        .map_err(|_| anyhow!("Failed to create index buffer"))?;
    Ok((vb, ib))
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
    /// Loads vertices, indices, UVs, and textures from a file as sub-meshes.
    pub fn load(
        path:          &str,
        ctx:           &StdRc<Context>,
        default_tex:   &Rc<Texture>,
        dynamic:       bool,
        texture_cache: &mut FxHashMap<String, Rc<Texture>>,
    ) -> Result<Self> {
        let scene = Scene::from_file(path)
            .map_err(|_| anyhow!("Failed to load scene: {path}"))?;

        // Materials
        let mut materials = Vec::with_capacity(scene.num_materials());
        for mat in scene.materials() {
            let mut m = MMaterial { tex: Rc::clone(default_tex) };
            if let Some(t) = mat.texture(Diffuse, 0) {
                let path = &t.path;
                if !texture_cache.contains_key(path) {
                    texture_cache.insert(path.clone(), Rc::new(Texture::from(path, ctx)?));
                }
                m.tex = Rc::clone(&texture_cache[path]);
            }
            materials.push(m);
        }

        // Geometry
        let mut vertices:   Vec<Vertex>  = vec![];
        let mut indices:    Vec<u32>     = vec![];
        let mut sub_meshes: Vec<SubMesh> = vec![];
        let mut v_offset:   u32          = 0;

        for mesh in scene.meshes() {
            let verts = mesh.vertices();
            let uvs: Vec<_> = mesh.texture_coords2(0)
                .filter(|c| c.len() == verts.len())
                .unwrap_or_else(|| vec![Vector2D { x: 0.0, y: 0.0 }; verts.len()]);

            let start = indices.len();
            mesh.faces().for_each(|f| {
                indices.extend(f.indices().iter().map(|&i| i + v_offset));
            });
            sub_meshes.push(SubMesh::new(start..indices.len(), mesh.material_index()));
            vertices.extend(uvs.iter().zip(verts.iter()).map(|(t, v)| Vertex {
                position: [v.x, v.y, v.z],
                uv:       [t.x, t.y],
            }));
            v_offset += verts.len() as u32;
        }

        // Embedded textures (stored, not yet bound to materials)
        let _embedded: Vec<_> = if scene.has_textures() {
            scene.textures().filter_map(|tex| {
                let (w, h) = tex.dimensions();
                tex.data().ok().and_then(|data| match data {
                    TextureData::Compressed(b) =>
                        image::load_from_memory(&b).ok().map(|i| i.to_rgba8()),
                    TextureData::Texels(t) => {
                        let raw: Vec<u8> = t.into_iter()
                            .flat_map(|p| [p.r, p.g, p.b, p.a])
                            .collect();
                        ImageBuffer::from_raw(w, h, raw)
                    }
                })
            }).collect()
        } else {
            vec![]
        };

        let bounds = Aabb::from_positions(vertices.iter().map(|v| v.position));
        let (v_buffer, i_buffer) = new_buffers(&vertices, &indices, ctx, dynamic)?;

        Ok(Self { v_buffer, i_buffer, sub_meshes, materials, bounds })
    }

    pub fn draw(
        &self,
        shader:    &Program,
        frame:     &mut Frame,
        cam:       &Camera,
        transform: &Transform,
        params:    &DrawParameters<'_>,
    ) -> Result<()> {
        if !transform.frustum_cull(&cam.planes, &self.bounds) { return Ok(()); }

        for sub in &self.sub_meshes {
            frame.draw(
                &self.v_buffer,
                self.i_buffer.slice(sub.range.clone())
                    .ok_or_else(|| anyhow!("Invalid index range: {:?}", sub.range))?,
                shader,
                &uniform! {
                    matrix:    transform.matrix_array,
                    vp:        cam.arr_vp,
                    texture2D: self.materials[sub.material].tex.data.sampled()
                        .wrap_function(glium::uniforms::SamplerWrapFunction::Repeat)
                        .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
                        .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
                },
                params,
            )?;
        }
        Ok(())
    }
}
