// Hint: include_str!("ruta/al/archivo") for loading files at comp-time
// std::env::current_dir() for fixing relative paths maybe
// FxHash vs AHash vs DashMap
// Maybe pointers to bypass hashing overhead when drawing
use std::{fs::read_to_string, vec, rc::Rc as StdRc};
use glam::Vec2;
use rclite::Rc;
use crate::modules::{imgui::{UI2D, Window2D}, render_3d::{aabb::Aabb, camera::Camera, model::{MCacheP, MMaterial, Model, ModelCache, SubMesh, Vertex, new_buffers}, transform::Transform}, texture::Texture};
use anyhow::{anyhow, Result};
use rustc_hash::FxHashMap;
use glium::{
    Blend, BlendingFunction, Depth, DepthTest, DrawParameters, Frame, LinearBlendingFactor, Program, backend::Context
};

pub struct ModelParams {
    pub transform: Transform,
}

pub struct Stage<'a> {
    // Only clippers would be an exception in a 3D space, but they should be handled differently anyway
    // draw_models(), _clippers(), _2D(), _UI(), etc. would be better, but for now this is fine
    // Adding also returns a unique ID, and should be used as the information will be lost otherwise. Doesn't incur to relationships, that has a separate matter.
    // Basically a Node hierarchy is a separate structure, irrelevant to the rendering process.
    pub models:    Vec<Model>,
    pub ctx:       StdRc<Context>,
    pub def_tex:   Rc<Texture>,
    pub cam:       Camera,
    pub ui:        UI2D,
    draw_params:   DrawParameters<'a>, 
    model_cache:   FxHashMap<String, Rc<ModelCache>>,
    shader_cache:  FxHashMap<String, Rc<Program>>,
    texture_cache: FxHashMap<String, Rc<Texture>>,
}

impl<'a> Stage<'a> {
    /// Initializes with model/shader/texture cache ignoring DoS attacks, a camera, DrawParameters settled with DepthTest + Alpha channel
    pub fn new(ctx: StdRc<Context>, def_tex: Rc<Texture>, dimensions: &Vec2) -> Self {
        Self {
            ui: UI2D { windows: vec![] },
            models:        vec![],
            model_cache:   FxHashMap::default(), shader_cache:  FxHashMap::default(), texture_cache: FxHashMap::default(),
            cam:           Camera::new(dimensions),
            ctx, def_tex,
            draw_params: DrawParameters {
                // Depth
                depth: Depth { test: DepthTest::IfLess, write: true, ..Default::default() },

                // Debug
                //polygon_mode: glium::PolygonMode::Line,
                //line_width: Some(2.0),

                // Alpha
                blend: Blend {
                    color: BlendingFunction::Addition {
                        source:      LinearBlendingFactor::SourceAlpha,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    },
                    alpha: BlendingFunction::Addition {
                        source:      LinearBlendingFactor::One,
                        destination: LinearBlendingFactor::OneMinusSourceAlpha,
                    },
                    constant_value: (0.0, 0.0, 0.0, 0.0),
                },
                backface_culling: glium::BackfaceCullingMode::CullClockwise,
                ..Default::default()
            },
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    #[allow(unused)]
    /// + Cache
    pub fn add_model(&mut self, path: &str, shader: &str, model: ModelParams, dynamic: bool) -> Result<usize> {
        let cache = self.get_model_cache(&path, dynamic); // (:
        let shader_p = self.get_shader(shader);
        self.models.push(Model { transform: model.transform, shader_p, cache: MCacheP::Shared((cache)) });
        Ok(self.models.len() - 1)
    }

    /// Creates if it doesn't exist, a shader has .vert + .frag files at assets/shaders
    pub fn get_shader(&mut self, shader: &str) -> Rc<Program> {
        let shader = format!("assets/shaders/{shader}");
        if !self.shader_cache.contains_key(&shader) { self.cache_shader(&shader).unwrap() }
        Rc::clone(&self.shader_cache[&shader])
    }

    /// A raw modification, no model caching
    #[allow(unused)]
    pub fn add_mesh(
        &mut self,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        materials: Vec<MMaterial>,
        sub_meshes: Vec<SubMesh>,
        shader: &str,
        model: ModelParams,
        dynamic: bool
    ) -> Result<usize> {
        let bounds = Aabb::from_positions(vertices.iter().map(|v| v.position));
        let (v_buffer, i_buffer) = new_buffers(&vertices, &indices, &self.ctx, dynamic)?;
        let cache_p = ModelCache { sub_meshes, materials, bounds, v_buffer, i_buffer };
        let shader_p = self.get_shader(shader);
        self.models.push(Model { transform: model.transform, shader_p, cache: MCacheP::Unique(cache_p)});
        Ok(self.models.len() - 1)
    }

    /// Creates if it doesn't exist
    pub fn get_model_cache(&mut self, path: &str, dynamic: bool) -> Rc<ModelCache> {
        let path   = format!("assets/models/{path}");
        if !self.model_cache.contains_key(&path) { self.cache_model(&path, dynamic).unwrap() }
        Rc::clone(&self.model_cache[&path])
    }

    #[allow(unused)]
    pub fn set_position(&mut self, id: usize, x: f32, y: f32, z: f32) -> Result<()> {
        self.models.get_mut(id)
            .ok_or_else(|| anyhow!("[{id}] Model not found"))?
            .set_position(x, y, z);
        Ok(())
    }

    #[allow(unused)]
    /// Returns a Copy of the bounds
    pub fn get_bounds(&self, path: &str) -> Result<Aabb> {
        self.model_cache.get(path)
            .map(|c| c.bounds)
            .ok_or_else(|| anyhow!("Model not cached: {path}"))
    }

    
    #[allow(unused)]
    /// Heavy operation that works with Dynamic Vertex Buffers
    pub fn update_mesh_buffers(&mut self, id: usize, vertices: Vec<Vertex>, indices: Vec<u32>, dynamic: bool) -> Result<()> {
        let MCacheP::Unique(cache) = &mut self.models[id].cache else {return Ok(())};
        let (v_buffer, i_buffer) = new_buffers(&vertices, &indices, &self.ctx, dynamic)?;
        cache.v_buffer = v_buffer;
        cache.i_buffer = i_buffer;
        Ok(())
    }

    pub fn add_window_2d(&mut self, window: Window2D) {
        self.ui.windows.push(window);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    #[allow(unused)]
    fn cache_model(&mut self, path: &str, dynamic: bool) -> Result<()> {
        let default_tex = Rc::clone(&self.def_tex);
        let cache = ModelCache::load(path, &self.ctx, &default_tex, dynamic, &mut self.texture_cache)?;
        self.model_cache.insert(path.into(), Rc::new(cache));
        Ok(())
    }

    fn cache_shader(&mut self, path: &str) -> Result<()> {
        if self.shader_cache.contains_key(path) { return Ok(()); }
        let vert = read_to_string(format!("{path}.vert"))
            .map_err(|_| anyhow!("Missing vertex shader: {path}.vert"))?;
        let frag = read_to_string(format!("{path}.frag"))
            .map_err(|_| anyhow!("Missing fragment shader: {path}.frag"))?;
        // Compile the shader files
        let program = Program::from_source(&self.ctx, &vert, &frag, None)
            .map_err(|e| anyhow!("Shader compile error: {e}"))?;
        self.shader_cache.insert(path.to_string(), Rc::new(program));
        Ok(())
    }

    // ── Render loop ───────────────────────────────────────────────────────────

    pub fn draw_models(&mut self, frame: &mut Frame) -> Result<()> {
        if self.models.is_empty() { return Ok(()); }
        for model in &self.models { model.draw(frame, &self.cam, &self.draw_params)? }
        Ok(())
    }
}