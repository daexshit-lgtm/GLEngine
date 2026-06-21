use anyhow::Result;
use imgui::Context;
use imgui_glow_renderer::{glow, Renderer};
use imgui_winit_support::WinitPlatform;
use glam::Vec2;
use glium::{
    Surface,
    backend::{Facade, glutin::SimpleWindowBuilder},
    glutin::{config::ConfigTemplateBuilder, display::GlDisplay, surface::WindowSurface},
};
use glium::glutin::display::{Display as GlutinDisplay, DisplayApiPreference};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{Window, WindowId},
};
use rclite::Rc;
use crate::modules::{stage::Stage, texture::Texture};

// ── Callback types ────────────────────────────────────────────────────────────
type StageFn<T> = Box<dyn Fn(&mut Stage, &mut T)              -> Result<()>>;
type KeyFn<T>   = Box<dyn Fn(&mut Stage, &mut T, KeyEvent)    -> Result<()>>;
type DevFn<T>   = Box<dyn Fn(&mut Stage, &mut T, DeviceEvent) -> Result<()>>;

// ── HandlerCallback ───────────────────────────────────────────────────────────
pub struct HandlerCallback<T> {
    on_init:      StageFn<T>,
    on_update:    StageFn<T>,
    on_key_press: KeyFn<T>,
    on_dev_event: DevFn<T>,
}

impl<T> HandlerCallback<T> {
    pub fn new() -> Self {
        Self {
            on_init:      Box::new(|_, _|    Ok(())),
            on_update:    Box::new(|_, _|    Ok(())),
            on_key_press: Box::new(|_, _, _| Ok(())),
            on_dev_event: Box::new(|_, _, _| Ok(())),
        }
    }

    /// Called once after initialization.
    pub fn with_init(mut self, f: impl Fn(&mut Stage, &mut T) -> Result<()> + 'static) -> Self {
        self.on_init = Box::new(f); self
    }
    /// Called once per frame.
    pub fn with_update(mut self, f: impl Fn(&mut Stage, &mut T) -> Result<()> + 'static) -> Self {
        self.on_update = Box::new(f); self
    }
    /// Called on keyboard input.
    pub fn with_key_press(mut self, f: impl Fn(&mut Stage, &mut T, KeyEvent) -> Result<()> + 'static) -> Self {
        self.on_key_press = Box::new(f); self
    }
    /// Called on raw device events (better for first-person camera).
    pub fn with_dev_event(mut self, f: impl Fn(&mut Stage, &mut T, DeviceEvent) -> Result<()> + 'static) -> Self {
        self.on_dev_event = Box::new(f); self
    }
}

// ── SystemState ───────────────────────────────────────────────────────────────
pub struct SystemState<'a> {
    pub display:  glium::Display<WindowSurface>,
    pub window:   Window,
    pub gl:       glow::Context,
    pub imgui:    Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub textures: imgui::Textures<glow::Texture>,
    stage:        Stage<'a>,
}

// ── App ───────────────────────────────────────────────────────────────────────
pub struct App<'a, T> {
    window:     Option<SystemState<'a>>,
    dimensions: Vec2,
    title:      String,
    state:      T,
    handlers:   HandlerCallback<T>,
}

impl<'a, T> ApplicationHandler for App<'a, T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let (window, display) = SimpleWindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(self.dimensions.x as u32, self.dimensions.y as u32)
            .with_config_template_builder(ConfigTemplateBuilder::new().with_multisampling(8))
            .build(event_loop);

        let init_target = display.draw();

        // Build a glow context sharing the glutin display's proc-address loader
        let raw_display = window.display_handle().unwrap().as_raw();
        let raw_window  = window.window_handle().unwrap().as_raw();
        let glutin_display = unsafe {
            GlutinDisplay::new(raw_display, DisplayApiPreference::EglThenWgl(Some(raw_window)))
                .expect("Failed to create native glutin display")
        };
        let ctx = display.get_context().clone();
        let gl  = unsafe {
            ctx.exec_in_context(|| {
                glow::Context::from_loader_function(|s| {
                    let cs = std::ffi::CString::new(s).unwrap();
                    glutin_display.get_proc_address(&cs) as *const _
                })
            })
        };

        // ImGui setup
        let mut imgui   = imgui::Context::create();
        imgui.set_ini_filename(None);
        let mut platform = WinitPlatform::new(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);
        let mut textures = imgui::Textures::<glow::Texture>::new();
        let renderer = Renderer::new(&gl, &mut imgui, &mut textures, true)
            .expect("Failed to init imgui-glow-renderer");

        init_target.finish().unwrap();

        // Stage
        let def_tex = Rc::new(
            Texture::from_rgba(&[255, 255, 255, 255], (1, 1), &ctx)
                .expect("Failed to create default texture"),
        );
        let mut stage = Stage::new(ctx, def_tex, &self.dimensions);
        (self.handlers.on_init)(&mut stage, &mut self.state).expect("on_init failed");

        self.window = Some(SystemState { display, window, gl, imgui, platform, renderer, textures, stage });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                let SystemState { textures, renderer, gl, window, platform, imgui, display, stage, .. }
                    = self.window.as_mut().unwrap();

                let mut frame = display.draw();
                frame.clear_color_and_depth((0.1, 0.2, 0.3, 1.0), 1.0);

                (self.handlers.on_update)(stage, &mut self.state).expect("on_update failed");
                stage.draw_models(&mut frame).expect("draw_models failed");

                platform.prepare_frame(imgui.io_mut(), window)
                    .expect("Failed to prepare ImGui frame");
                stage.ui.draw(imgui).expect("draw ui failed");

                renderer.render(gl, textures, imgui.render())
                    .expect("Failed to render ImGui");
                frame.finish().expect("Failed to finish frame");
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let SystemState { stage, .. } = self.window.as_mut().unwrap();
                (self.handlers.on_key_press)(stage, &mut self.state, event)
                    .expect("on_key_press failed");
            }

            WindowEvent::Resized(s) => {
                let dim = Vec2::new(s.width as f32, s.height as f32);
                let SystemState { stage, .. } = self.window.as_mut().unwrap();
                stage.cam.dimensions = dim;
                stage.cam.update_dimensions();
                self.dimensions = dim;
            }

            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let SystemState { stage, .. } = self.window.as_mut().unwrap();
        (self.handlers.on_dev_event)(stage, &mut self.state, event)
            .expect("on_dev_event failed");
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.window.as_ref().unwrap().window.request_redraw();
    }
}

impl<'a, T> App<'a, T> {
    pub fn run(dimensions: Vec2, title: String, state: T, handlers: HandlerCallback<T>) -> Result<()> {
        EventLoop::new()?.run_app(&mut Self { window: None, handlers, title, state, dimensions })?;
        Ok(())
    }
}
