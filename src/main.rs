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

use crate::modules::{app::{App, HandlerCallback}, imgui::{Data2D, Window2D}, render_3d::model::{MMaterial, SubMesh, Vertex}, stage::ModelParams, texture::Texture};

// ── Chunk constants ───────────────────────────────────────────────────────────
const CHUNK_W:    i32 = 16;
const CHUNK_H:    i32 = 256;
const CHUNK_VOL:  usize = (CHUNK_W * CHUNK_H * CHUNK_W) as usize;
const SEA_LEVEL:  i32 = 64;
const HEIGHT_MIN: i32 = 60;
const HEIGHT_RANGE: f64 = 40.0;

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

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: Block) {
        if x < CHUNK_W && y < CHUNK_H && z < CHUNK_W {
            self.data[Self::idx(x, y, z)] = b as u8;
        }
    }

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
            let gx   = ox + x as f64;
            let gz   = oz + z as f64;
            let biome = biome_for(
                perlin.get([gx * 0.01, gz * 0.01, 5.0]),
                perlin.get([gx * 0.01, gz * 0.01, 12.0]),
            );
            let top = HEIGHT_MIN
                + ((perlin.get([gx * 0.03, gz * 0.03, 0.0]) + 1.0) / 2.0 * HEIGHT_RANGE) as i32;

            for y in 0..CHUNK_H {
                let block = match y {
                    0                    => Block::Bedrock,
                    y if y < top - 4    => Block::Stone,
                    y if y < top - 1    => biome.subsurface,
                    y if y == top - 1   => biome.surface,
                    y if y < SEA_LEVEL  => Block::Water,
                    _                   => Block::Air,
                };
                chunk.set(x, y, z, block);
            }
        }
    }
}

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

const CUBE_VERTICES: [IVec3; 8] = [
    IVec3::new(0, 0, 1), IVec3::new(1, 0, 1), IVec3::new(1, 1, 1), IVec3::new(0, 1, 1), // Front
    IVec3::new(0, 0, 0), IVec3::new(1, 0, 0), IVec3::new(1, 1, 0), IVec3::new(0, 1, 0), // Back
];


// Triangle faces
const QUAD_INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

// Index mapping/connecting to CUBE_VERTICES; 4 vertices per face forming a rectangle
const FACE_VERTICES: [[usize; 4]; 6] = [
    [0, 1, 2, 3], // Front
    [5, 4, 7, 6], // Back
    [4, 0, 3, 7], // Left
    [1, 5, 6, 2], // Right
    [3, 2, 6, 7], // Up
    [4, 5, 1, 0], // Down
];

/// Block Directions to seek Neighbors
const NEIGHBORS: [IVec3; 6] = [
    IVec3::new(0, 0, 1),  // Front (+Z)
    IVec3::new(0, 0, -1), // Back (-Z)
    IVec3::new(-1, 0, 0), // Left (-X)
    IVec3::new(1, 0, 0),  // Right (+X)
    IVec3::new(0, 1, 0),  // Up (+Y)
    IVec3::new(0, -1, 0), // Down (-Y)
];

const UVS: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [1.0, 1.0],
    [0.0, 1.0],
];

const FACE_UV_OFFSET: [(f32, f32, f32); 6] = {
    const M: f32 = 0.33333334;
    const Z: f32 = 0.0;
    [
        (M, M, M), // Front
        (M, M, M), // Back
        (M, M, M), // Left
        (M, M, M), // Right
        (M, Z, M), // Up
        (M, Z, Z), // Down
    ]
};

const C_LOOP: i32 = 20;

fn main() {
    let handlers = HandlerCallback::new()
    .with_init(|stage: &mut Stage, _: &mut State| {
        stage.cam.set_far(2000.0);
        stage.cam.speed = 1.0;
        stage.add_window_2d(Window2D::new("Test".into(), vec![Data2D::Button, Data2D::Button]));
        mc(stage)
    })
    // FPS
    .with_update(|_, state| { fps(state); Ok(()) })
    .with_key_press(|stage, _, key| { on_key(stage, key);  Ok(()) })
    .with_dev_event(|stage, _, dev| { on_mouse(stage, dev); Ok(()) });

    App::run(Vec2::new(800.0, 600.0), "GLEngine".into(), State { last_fps: Instant::now(), frames: 0 }, handlers).unwrap();
}

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

type ChunkMap = FxHashMap<(i32, i32), Chunk>;

/// The Game Logic
fn mc(stage: &mut Stage) -> Result<()> {
    let perlin = Perlin::new(42);
    let grass_texture = Rc::new(Texture::from("TSGrassCube.jpg", &stage.ctx)?);
    let chunk_materials = vec![MMaterial { tex: grass_texture }];
    
    // Generate
    let mut chunks: ChunkMap = FxHashMap::default();
    for cz in 0..C_LOOP {
        for cx in 0..C_LOOP {
            let mut chunk = Chunk::new(cx, cz);
            generate(&mut chunk, &perlin);
            chunks.insert((cx, cz), chunk);
        }
    }

    for cz in 0..C_LOOP {
        for cx in 0..C_LOOP {
            let chunk = &chunks[&(cx, cz)];
            let mut vertices: Vec<Vertex> = Vec::with_capacity(1024);
            let mut indices: Vec<u32> = Vec::with_capacity(3072);

            // Filtering Visibility of Blocks
            for y in 0..CHUNK_H {
                for z in 0..CHUNK_W {
                    for x in 0..CHUNK_W {
                        if chunk.get(x, y, z) != Block::Air { filter_faces(
                            IVec3::new(x, y, z),
                            &mut vertices,
                            &mut indices,
                            IVec2::new(cx, cz),
                            &chunks,
                            &chunk
                        ) }
                    }
                }
            }

            let sub_meshes = vec![SubMesh::new(0..indices.len(), 0)];
            
            stage.add_mesh(
                vertices, 
                indices, 
                chunk_materials.clone(), // Only this is cloned
                sub_meshes,
                "default",
                ModelParams {
                    transform: Transform::new((cx * CHUNK_W) as f32, -70.0, (cz * CHUNK_W) as f32),
                },
                false // Dynamic when breaking a block
            )?;
        }
    }
    Ok(())
}

fn block_is_visible(n: IVec3, c: IVec2, chunks: &ChunkMap, chunk: &Chunk) -> bool {
    let upside_x = n.x < 0;
    let downside_x = n.x >= CHUNK_W;
    let upside_z = n.z < 0;
    let downside_z = n.z >= CHUNK_W;
    let outside_x = upside_x || downside_x;
    let outside_z = upside_z || downside_z;
    if n.y < 0 { true } // Upside
    else if outside_x || outside_z {
        let mut neighbor = IVec2::new(c.x, c.y);
        let mut local = IVec2::new(n.x, n.z);

        if upside_x { neighbor.x -= 1; local.x += CHUNK_W; }
        else if downside_x { neighbor.x += 1; local.x -= CHUNK_W; }

        if upside_z { neighbor.y -= 1; local.y += CHUNK_W; }
        else if downside_z { neighbor.y += 1; local.y -= CHUNK_W; }

        if let Some(neighbor_chunk) = chunks.get(&(neighbor.x, neighbor.y)) {
            neighbor_chunk.get(local.x, n.y, local.y) == Block::Air // Chunk beside
        } else {
            false // End of the map
        }
    } else { chunk.get(n.x, n.y, n.z) == Block::Air } // Inside the chunk(Still renders)
}

/// Filtering faces intercepting other chunks
fn filter_faces(
    pos: IVec3,
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    c: IVec2,
    chunks: &ChunkMap,
    chunk: &Chunk
) {
    for face_idx in 0..6 {
        if !block_is_visible(pos + NEIGHBORS[face_idx], c, chunks, chunk) { continue }
        let v_offset = vertices.len() as u32;

        for (i, &v_idx) in FACE_VERTICES[face_idx].iter().enumerate() {
            let mut uv = UVS[i];

            // UV
            let (mul, u_add, v_add) = FACE_UV_OFFSET[face_idx];
            uv[0] = uv[0] * mul + u_add;
            uv[1] = uv[1] * mul + v_add;


            let v_pos = pos + CUBE_VERTICES[v_idx];
            vertices.push(Vertex {
                position: [
                    v_pos.x as f32,
                    v_pos.y as f32,
                    v_pos.z as f32,
                ],
                uv,
            });
        }

        // Add
        for &i in &QUAD_INDICES { indices.push(v_offset + i); }
    }
}