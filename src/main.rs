mod modules;

use std::time::Instant;

use anyhow::Result;
use rustc_hash::FxHashMap;
use glam::{IVec2, IVec3, Vec2, Vec3};
use modules::{stage::Stage, render_3d::transform::Transform};
use noise::{NoiseFn, Perlin};
use rclite::Rc;
use winit::{
    event::{DeviceEvent, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use crate::modules::{
    app::{App, HandlerCallback},
    imgui::{Data2D, Window2D},
    render_3d::model::{MMaterial, SubMesh, Vertex},
    stage::ModelParams,
    texture::Texture,
};

// ── Chunk constants ───────────────────────────────────────────────────────────
const CHUNK_W:     i32   = 16;
const CHUNK_H:     i32   = 256;
const CHUNK_VOL:   usize = (CHUNK_W * CHUNK_H * CHUNK_W) as usize;
const SEA_LEVEL:   i32   = 64;
const HEIGHT_MIN:  i32   = 60;
const HEIGHT_RANGE: f64  = 40.0;
const C_LOOP:      i32   = 20;

// ── Block ─────────────────────────────────────────────────────────────────────
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Block { Air = 0, Stone, Dirt, Grass, Sand, Water, Bedrock }

impl Block {
    #[inline]
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Stone,  2 => Self::Dirt, 3 => Self::Grass,
            4 => Self::Sand,   5 => Self::Water, 6 => Self::Bedrock,
            _ => Self::Air,
        }
    }
}

// ── Chunk ─────────────────────────────────────────────────────────────────────
pub struct Chunk {
    data: Box<[u8; CHUNK_VOL]>,
    cx:   i32,
    cz:   i32,
}

impl Chunk {
    pub fn new(cx: i32, cz: i32) -> Self {
        Self { data: Box::new([0u8; CHUNK_VOL]), cx, cz }
    }

    #[inline(always)]
    fn idx(x: i32, y: i32, z: i32) -> usize {
        (x * CHUNK_H * CHUNK_W + y * CHUNK_W + z) as usize
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, b: Block) {
        if x < CHUNK_W && y < CHUNK_H && z < CHUNK_W {
            self.data[Self::idx(x, y, z)] = b as u8;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if x < CHUNK_W && y < CHUNK_H && z < CHUNK_W {
            Block::from_u8(self.data[Self::idx(x, y, z)])
        } else {
            Block::Air
        }
    }
}

// ── Biome ─────────────────────────────────────────────────────────────────────
struct Biome { surface: Block, subsurface: Block }

#[inline]
fn biome_for(temp: f64, humidity: f64) -> Biome {
    if temp > 0.4 && humidity < -0.3 {
        Biome { surface: Block::Sand,  subsurface: Block::Sand }
    } else {
        Biome { surface: Block::Grass, subsurface: Block::Dirt }
    }
}

// ── World generation ──────────────────────────────────────────────────────────
pub fn generate(chunk: &mut Chunk, perlin: &Perlin) {
    let ox = (chunk.cx * CHUNK_W) as f64;
    let oz = (chunk.cz * CHUNK_W) as f64;

    for x in 0..CHUNK_W {
        for z in 0..CHUNK_W {
            let gx = ox + x as f64;
            let gz = oz + z as f64;

            let biome = biome_for(
                perlin.get([gx * 0.01, gz * 0.01, 5.0]),
                perlin.get([gx * 0.01, gz * 0.01, 12.0]),
            );
            let top = HEIGHT_MIN
                + ((perlin.get([gx * 0.03, gz * 0.03, 0.0]) + 1.0) / 2.0 * HEIGHT_RANGE) as i32;

            for y in 0..CHUNK_H {
                let block = match y {
                    0                  => Block::Bedrock,
                    y if y < top - 4   => Block::Stone,
                    y if y < top - 1   => biome.subsurface,
                    y if y == top - 1  => biome.surface,
                    y if y < SEA_LEVEL => Block::Water,
                    _                  => Block::Air,
                };
                chunk.set(x, y, z, block);
            }
        }
    }
}

// ── Geometry constants ────────────────────────────────────────────────────────
const CUBE_VERTICES: [IVec3; 8] = [
    IVec3::new(0, 0, 1), IVec3::new(1, 0, 1), IVec3::new(1, 1, 1), IVec3::new(0, 1, 1), // Front
    IVec3::new(0, 0, 0), IVec3::new(1, 0, 0), IVec3::new(1, 1, 0), IVec3::new(0, 1, 0), // Back
];

/// Two triangles forming a quad
const QUAD_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

/// 4 vertex indices per face, into CUBE_VERTICES
const FACE_VERTICES: [[usize; 4]; 6] = [
    [0, 1, 2, 3], // Front
    [5, 4, 7, 6], // Back
    [4, 0, 3, 7], // Left
    [1, 5, 6, 2], // Right
    [3, 2, 6, 7], // Up
    [4, 5, 1, 0], // Down
];

/// Neighbor offsets per face (matches FACE_VERTICES order)
const NEIGHBORS: [IVec3; 6] = [
    IVec3::new( 0,  0,  1), // Front (+Z)
    IVec3::new( 0,  0, -1), // Back  (-Z)
    IVec3::new(-1,  0,  0), // Left  (-X)
    IVec3::new( 1,  0,  0), // Right (+X)
    IVec3::new( 0,  1,  0), // Up    (+Y)
    IVec3::new( 0, -1,  0), // Down  (-Y)
];

const UVS: [[f32; 2]; 4] = [
    [0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0],
];

/// Per-face UV atlas offset: (scale, u_add, v_add)
const FACE_UV_OFFSET: [(f32, f32, f32); 6] = {
    const M: f32 = 1.0 / 3.0;
    [
        (M, M, M), // Front
        (M, M, M), // Back
        (M, M, M), // Left
        (M, M, M), // Right
        (M, 0.0, M), // Up
        (M, 0.0, 0.0), // Down
    ]
};

// ── Input handlers ────────────────────────────────────────────────────────────
fn on_key(stage: &mut Stage, key: KeyEvent) {
    let PhysicalKey::Code(k) = key.physical_key else { return };
    let delta = match k {
        KeyCode::KeyW => Vec3::new( 1.0,  0.0,  0.0),
        KeyCode::KeyS => Vec3::new(-1.0,  0.0,  0.0),
        KeyCode::KeyD => Vec3::new( 0.0,  0.0,  1.0),
        KeyCode::KeyA => Vec3::new( 0.0,  0.0, -1.0),
        KeyCode::KeyE => Vec3::new( 0.0,  1.0,  0.0),
        KeyCode::KeyQ => Vec3::new( 0.0, -1.0,  0.0),
        _             => return,
    };
    stage.cam.move_by(delta);
}

fn on_mouse(stage: &mut Stage, event: DeviceEvent) {
    if let DeviceEvent::MouseMotion { delta } = event {
        stage.cam.rotate_by(Vec2::new(delta.0 as f32, delta.1 as f32));
    }
}

// ── App state ─────────────────────────────────────────────────────────────────
struct State { last_fps: Instant, frames: u32 }

fn fps(state: &mut State) {
    state.frames += 1;
    let elapsed = state.last_fps.elapsed();
    if elapsed.as_secs() >= 1 {
        let secs = elapsed.as_secs_f32();
        println!("[Engine] FPS: {:.2} | Frame: {:.2}ms",
            state.frames as f32 / secs,
            secs / state.frames as f32 * 1000.0,
        );
        state.frames   = 0;
        state.last_fps = Instant::now();
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
fn main() {
    App::run(
        Vec2::new(800.0, 600.0),
        "GLEngine".into(),
        State { last_fps: Instant::now(), frames: 0 },
        HandlerCallback::new()
            .with_init(|stage: &mut Stage, _: &mut State| {
                stage.cam.set_far(2000.0);
                stage.cam.speed = 1.0;
                stage.add_window_2d(Window2D::new("Test".into(), vec![Data2D::Button, Data2D::Button]));
                mc(stage)
            })
            .with_update   (|_,     state| { fps(state); Ok(()) })
            .with_key_press(|stage, _, key| { on_key(stage, key);   Ok(()) })
            .with_dev_event(|stage, _, dev| { on_mouse(stage, dev); Ok(()) }),
    ).unwrap();
}

// ── Chunk map type ────────────────────────────────────────────────────────────
type ChunkMap = FxHashMap<(i32, i32), Chunk>;

/// World generation + mesh building
fn mc(stage: &mut Stage) -> Result<()> {
    let perlin          = Perlin::new(42);
    let grass_texture   = Rc::new(Texture::from("TSGrassCube.jpg", &stage.ctx)?);
    let chunk_materials = vec![MMaterial { tex: grass_texture }];

    // Generate all chunks first
    let mut chunks: ChunkMap = FxHashMap::default();
    chunks.reserve((C_LOOP * C_LOOP) as usize);
    for cz in 0..C_LOOP {
        for cx in 0..C_LOOP {
            let mut chunk = Chunk::new(cx, cz);
            generate(&mut chunk, &perlin);
            chunks.insert((cx, cz), chunk);
        }
    }

    // Build meshes
    for cz in 0..C_LOOP {
        for cx in 0..C_LOOP {
            let mut vertices: Vec<Vertex> = Vec::with_capacity(8192);
            let mut indices:  Vec<u32>    = Vec::with_capacity(12288);
            let pos           = IVec2::new(cx, cz);
            let chunk         = &chunks[&(cx, cz)];

            for y in 0..CHUNK_H {
                for z in 0..CHUNK_W {
                    for x in 0..CHUNK_W {
                        if chunk.get(x, y, z) != Block::Air {
                            filter_faces(IVec3::new(x, y, z), &mut vertices, &mut indices, pos, &chunks, chunk);
                        }
                    }
                }
            }

            if vertices.is_empty() { continue; }

            let sub_meshes = vec![SubMesh::new(0..indices.len(), 0)];
            stage.add_mesh(
                vertices,
                indices,
                chunk_materials.clone(),
                sub_meshes,
                "default",
                ModelParams {
                    transform: Transform::new((cx * CHUNK_W) as f32, -70.0, (cz * CHUNK_W) as f32),
                },
                false,
            )?;
        }
    }
    Ok(())
}

// ── Face culling helpers ──────────────────────────────────────────────────────

/// Returns true if the block at `n` (world-local coords) is transparent/air,
/// meaning the current face should be rendered.
fn block_is_visible(n: IVec3, c: IVec2, chunks: &ChunkMap, chunk: &Chunk) -> bool {
    if n.y < 0 { return true; } // Below bedrock → always draw bottom face

    let ox = (n.x < 0) as i32 - (n.x >= CHUNK_W) as i32; // -1, 0, or +1
    let oz = (n.z < 0) as i32 - (n.z >= CHUNK_W) as i32;

    if ox == 0 && oz == 0 {
        // Same chunk
        chunk.get(n.x, n.y, n.z) == Block::Air
    } else {
        // Cross-chunk lookup
        let ncx = c.x - ox;
        let ncz = c.y - oz;
        let lx  = n.x + ox * CHUNK_W;
        let lz  = n.z + oz * CHUNK_W;
        chunks.get(&(ncx, ncz))
            .map(|nc| nc.get(lx, n.y, lz) == Block::Air)
            .unwrap_or(false) // Edge of the map → don't render
    }
}

/// Emits vertices/indices for every visible face of the block at `pos`.
fn filter_faces(
    pos:      IVec3,
    vertices: &mut Vec<Vertex>,
    indices:  &mut Vec<u32>,
    c:        IVec2,
    chunks:   &ChunkMap,
    chunk:    &Chunk,
) {
    for face_idx in 0..6usize {
        if !block_is_visible(pos + NEIGHBORS[face_idx], c, chunks, chunk) { continue; }

        let v_offset = vertices.len() as u32;

        for (i, &v_idx) in FACE_VERTICES[face_idx].iter().enumerate() {
            let (mul, u_add, v_add) = FACE_UV_OFFSET[face_idx];
            let raw_uv = UVS[i];
            let uv = [raw_uv[0] * mul + u_add, raw_uv[1] * mul + v_add];

            let vp = pos + CUBE_VERTICES[v_idx];
            vertices.push(Vertex {
                position: [vp.x as f32, vp.y as f32, vp.z as f32],
                uv,
            });
        }

        for &i in &QUAD_INDICES { indices.push(v_offset + i); }
    }
}
