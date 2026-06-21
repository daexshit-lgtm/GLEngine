use anyhow::Result;
use dear_imgui_rs::{BackendFlags, Context};
use glam::Vec2;
use glium::{Display, Surface, backend::{Facade, glutin::SimpleWindowBuilder}, glutin::{config::ConfigTemplateBuilder, surface::WindowSurface}};
use winit::{
    application::ApplicationHandler, event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}
};

use crate::modules::{render_3d::texture::Texture, stage::Stage};
use rclite::Rc;

// ── Callback types ────────────────────────────────────────────────────────────
type StageFn<T> = Box<dyn Fn(&mut Stage, &mut T)                -> Result<()>>;
type KeyFn<T>   = Box<dyn Fn(&mut Stage, &mut T, KeyEvent)      -> Result<()>>;
type DevFn<T>   = Box<dyn Fn(&mut Stage, &mut T, DeviceEvent)   -> Result<()>>;

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
            on_init:      Box::new(|_,_| Ok(())),
            on_update:    Box::new(|_,_| Ok(())),
            on_key_press: Box::new(|_,_,_| Ok(())),
            on_dev_event: Box::new(|_,_,_| Ok(())),
        }
    }

    /// Once right after initialization
    pub fn with_init     (mut self, f: impl Fn(&mut Stage, &mut T) -> Result<()> + 'static)              -> Self { self.on_init      = Box::new(f); self }
    /// Execute only at key press
    pub fn with_key_press(mut self, f: impl Fn(&mut Stage, &mut T, KeyEvent) -> Result<()> + 'static)    -> Self { self.on_key_press = Box::new(f); self }
    /// Execute only at a "device" event
    /// 
    /// Better implementation for first-person camera
    pub fn with_dev_event(mut self, f: impl Fn(&mut Stage, &mut T, DeviceEvent) -> Result<()> + 'static) -> Self { self.on_dev_event = Box::new(f); self }
    /// Execute per frame
    pub fn with_update   (mut self, f: impl Fn(&mut Stage, &mut T) -> Result<()> + 'static)              -> Self { self.on_update    = Box::new(f); self }
}
// ── App (winit handler) ───────────────────────────────────────────────────────
pub struct App<'a, T> {
    window: Option<(Window, Display<WindowSurface>, Stage<'a>)>,
    dimensions: Vec2,
    title:      String,
    state:      T,
    handlers:   HandlerCallback<T>,

    imgui: Option<dear_imgui_rs::Context>,
}

/// Cross-Platform Events
impl<'a, T> ApplicationHandler for App<'a, T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let dimensions = &self.dimensions;
        let config_template = ConfigTemplateBuilder::new()
            .with_multisampling(8); // 4x MSAA
        let (window, display) = SimpleWindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(dimensions.x as u32, dimensions.y as u32)
              .with_config_template_builder(config_template)
            .build(event_loop);
        let mut imgui = Context::create();
        imgui.io_mut().set_display_size([
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        ]);
        imgui.io_mut().set_backend_flags(BackendFlags::all());
        self.imgui = Some(imgui);
        let ctx = display.get_context().clone();
        let def_tex = Rc::new(
            Texture::from_rgba(&[255, 255, 255, 255], (1, 1), &ctx).expect("Failed to create default texture"),
        );
        let mut stage = Stage::new(ctx, def_tex, &dimensions);
        (self.handlers.on_init)(&mut stage, &mut self.state).expect("on_init failed");
        self.window = Some((window, display, stage));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let (_, display, stage) = self.window.as_mut().unwrap();
                let imgui = self.imgui.as_mut().unwrap(); // Collapse ^
                let (mut frame, ui) = (display.draw(), imgui.frame());
                // TODO: BG Color
                frame.clear_color_and_depth((0.1, 0.2, 0.3, 1.0), 1.0);
                (self.handlers.on_update)(stage, &mut self.state).expect("on_update failed");
                stage.draw_models(&mut frame).expect("draw failed");
                frame.finish().expect("frame finish failed");
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let (_, _, stage) = self.window.as_mut().unwrap();
                (self.handlers.on_key_press)(stage, &mut self.state, event).expect("on_key_press failed")
            }
            WindowEvent::Resized(s) => {
                let dimensions = Vec2::new(s.width as f32, s.height as f32);
                let (_, _, stage) = self.window.as_mut().unwrap();
                stage.cam.dimensions = dimensions;
                stage.cam.update_dimensions();
                self.dimensions = dimensions;
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let (_, _, stage) = self.window.as_mut().unwrap();
        (self.handlers.on_dev_event)(stage, &mut self.state, event).expect("on_dev_event failed");
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        let (window, _, _) = self.window.as_ref().unwrap();
        window.request_redraw();
    }
}

impl<'a, T> App<'a, T> {
    pub fn run(dimensions: Vec2, title: String, state: T, handlers: HandlerCallback<T>) -> Result<()> {
        EventLoop::new()?.run_app(&mut Self {
            window: None, imgui: None,
            handlers, title, state, dimensions
        })?;
        Ok(())
    }
}