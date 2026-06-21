use anyhow::Result;
use imgui::Context;
use imgui_glow_renderer::{Renderer, glow};
use imgui_glow_renderer::Renderer as GlowRenderer;
use imgui_winit_support::WinitPlatform;
use glam::Vec2;
use glium::{Surface, backend::{Facade, glutin::SimpleWindowBuilder}, glutin::{config::ConfigTemplateBuilder, display::GlDisplay, surface::WindowSurface}};
use glium::glutin::display::{Display as GlutinDisplay, DisplayApiPreference};
use winit::{
    application::ApplicationHandler, event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, raw_window_handle::{HasDisplayHandle, HasWindowHandle}, window::{Window, WindowId}
};

use crate::modules::{stage::Stage, texture::Texture};
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


pub struct SystemState<'a> {
    pub display: glium::Display<WindowSurface>,
    pub window: Window,
    pub gl: glow::Context, // Glow viviendo feliz dentro del contexto de Glium
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    stage: Stage<'a>,
    textures: imgui::Textures<glow::Texture>
}

// ── App (winit handler) ───────────────────────────────────────────────────────
pub struct App<'a, T> {
    window:     Option<SystemState<'a>>,
    dimensions: Vec2,
    title:      String,
    state:      T,
    handlers:   HandlerCallback<T>,
}

/// Cross-Platform Events
impl<'a, T> ApplicationHandler for App<'a, T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let dimensions = &self.dimensions;
        let config_template = ConfigTemplateBuilder::new()
            .with_multisampling(8); // 8x MSAA fantástico

        let (window, display) = SimpleWindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(dimensions.x as u32, dimensions.y as u32)
            .with_config_template_builder(config_template)
            .build(event_loop);

        let raw_display_handle = window.display_handle().unwrap().as_raw();
        let raw_window_handle = window.window_handle().unwrap().as_raw();

        let preference = DisplayApiPreference::EglThenWgl(Some(raw_window_handle));
        let glutin_display = unsafe {
            GlutinDisplay::new(raw_display_handle, preference)
                .expect("Fallo al crear el display nativo de glutin")
        };
        
        let ctx = display.get_context().clone();

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::new(&mut imgui); // Corregido: .init en vez de .new
        platform.attach_window(
            imgui.io_mut(), 
            &window, 
            imgui_winit_support::HiDpiMode::Default
        );

        let gl = unsafe {
            glow::Context::from_loader_function(|s| {
                let c_str = format!("{}\0", s);
                let c_str_ptr = std::ffi::CStr::from_bytes_with_nul(c_str.as_bytes()).unwrap();
                glutin_display.get_proc_address(c_str_ptr) as *const _
            })
        };

        let mut textures = imgui::Textures::<glow::Texture>::new();

        let renderer = GlowRenderer::new(&gl, &mut imgui, &mut textures, true)
            .expect("Fallo al inicializar imgui-glow-renderer");

        let def_tex = Rc::new(
            Texture::from_rgba(&[255, 255, 255, 255], (1, 1), &ctx).expect("Failed to create default texture"),
        );

        let mut stage = Stage::new(ctx, def_tex, &dimensions);
        (self.handlers.on_init)(&mut stage, &mut self.state).expect("on_init failed");
        
        self.window = Some(SystemState {
            display,
            window,
            gl, 
            imgui,
            platform,
            renderer,
            stage,
            textures
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let SystemState{textures,renderer, gl, window, platform, imgui, display, stage,..} = self.window.as_mut().unwrap();
                let mut frame = display.draw();
                // TODO: BG Color
                frame.clear_color_and_depth((0.1, 0.2, 0.3, 1.0), 1.0);
                (self.handlers.on_update)(stage, &mut self.state).expect("on_update failed");
                stage.draw_models(&mut frame).expect("draw failed");
                platform.prepare_frame(imgui.io_mut(), &window)
                    .expect("Fallo al preparar frame de ImGui");
                stage.ui.draw(imgui).expect("draw ui failed");
let draw_data = imgui.render();
renderer.render(&gl, textures, draw_data)
    .expect("Error al renderizar ImGui con Glow");
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let SystemState { stage, .. } = self.window.as_mut().unwrap();
                (self.handlers.on_key_press)(stage, &mut self.state, event).expect("on_key_press failed")
            }
            WindowEvent::Resized(s) => {
                let dimensions = Vec2::new(s.width as f32, s.height as f32);
                let SystemState { stage, .. } = self.window.as_mut().unwrap();
                stage.cam.dimensions = dimensions;
                stage.cam.update_dimensions();
                self.dimensions = dimensions;
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let SystemState { stage, .. } = self.window.as_mut().unwrap();
        (self.handlers.on_dev_event)(stage, &mut self.state, event).expect("on_dev_event failed");
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        let SystemState { window, .. } = self.window.as_mut().unwrap();
        window.request_redraw();
    }
}

impl<'a, T> App<'a, T> {
    pub fn run(dimensions: Vec2, title: String, state: T, handlers: HandlerCallback<T>) -> Result<()> {
        EventLoop::new()?.run_app(&mut Self { window: None, handlers, title, state, dimensions })?;
        Ok(())
    }
}