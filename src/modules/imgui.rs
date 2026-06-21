use anyhow::Result;
use imgui::Context as ImGuiContext;

pub enum Data2D { Button }

pub struct Window2D {
    title: String,
    data:  Vec<Data2D>,
}

impl Window2D {
    pub fn new(title: String, data: Vec<Data2D>) -> Self {
        Self { title, data }
    }
}

pub struct UI2D {
    pub windows: Vec<Window2D>,
}

impl UI2D {
    pub fn draw(&mut self, imgui: &mut ImGuiContext) -> Result<()> {
        let ui = imgui.frame();
        for w in &self.windows {
            ui.window(&w.title).build(|| {
                ui.text("Hello ImGui");
                for d in &w.data {
                    match d {
                        Data2D::Button => { ui.button("Play"); }
                    }
                }
            });
        }
        Ok(())
    }
}
