use std::rc::Rc;

use anyhow::Result;
use glium::{Frame, Program, Surface, backend::Context, implement_vertex};
use dear_imgui_rs::{Context as ImGuiContext, Ui};

use crate::modules::render_3d::{model::new_buffers, texture::Texture};

struct Transform2D {
}

struct Button2D {
    transform: Transform2D
}

enum Data {
    Button
}

struct Window2D {
    title: String,
    data: Vec<Data>
}

#[derive(Default)]
pub struct UI2D {
    windows: Vec<Window2D>
}

impl UI2D {
    fn draw(
        &self,
        frame:      &mut Frame,
        ui: &Ui,
        imgui: &mut ImGuiContext,
        //ctx: &Rc<Context>,
        //shader: &Program
    ) -> Result<()> {
        for w in &self.windows {
            ui.window(&w.title)
                .build(|| {
                    ui.text("Hello ImGui");
                    for d in &w.data {
                        match d {
                            Data::Button => ui.button("Play")
                        };
                    }
                });
        }
        let draw_data = imgui.render();

println!(
    "{} draw lists",
    draw_data.draw_lists().count()
);
/* 
        for draw_list in imgui.render().draw_lists() {
            for cmd in draw_list.commands() {
                let vertices: Vec<ImGuiVertex> =
                    draw_list.vtx_buffer()
                        .iter()
                        .map(|v| ImGuiVertex {
                            pos: v.pos,
                            uv: v.uv,
                            col: v.col,
                        })
                        .collect();
                let indices =
                    draw_list.idx_buffer();
                let fonts = imgui.fonts();
                // TODO: Dynamic
                let (verts, inds) = &new_buffers(&vertices, &indices, ctx, false)?;

                frame.draw(verts, inds, shader, uniforms, draw_parameters);
            }
        }
    */
        Ok(())
    }
}

#[derive(Copy, Clone)]
struct ImGuiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    col: u32,
}

implement_vertex!(ImGuiVertex, pos, uv, col);