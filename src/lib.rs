use std::sync::Arc;
use wgpu::*;
use winit::{application::*, dpi::PhysicalSize, event::*, event_loop::*, window::*};

//
// Irrelevant utility shizzle
//
pub trait LogResult {
    type Result;
    fn log_result(self, msg: &str) -> Self::Result;
}
impl<T, E: std::fmt::Debug> LogResult for Result<T, E> {
    type Result = T;
    fn log_result(self, msg: &str) -> Self::Result {
        match self {
            Ok(x) => {
                log::info!("{msg} succeeded ({}:{})", file!(), line!());
                x
            }
            Err(err) => {
                log::error!("{msg} failed: {err:?} ({}:{})", file!(), line!());
                panic!("panic!() caused by unexpected (and unhandled) error: {err:?}");
            }
        }
    }
}
impl<T> LogResult for Option<T> {
    type Result = T;
    fn log_result(self, msg: &str) -> Self::Result {
        if let Some(x) = self {
            log::info!("{msg} succeeded ({}:{})", file!(), line!());
            x
        } else {
            log::error!("{msg} returned `None` ({}:{})", file!(), line!());
            panic!("panic!() caused by unexpected (and unhandled) None");
        }
    }
}

macro_rules! log_result {
    ($e:expr) => {{
        use $crate::LogResult;
        $e.log_result(stringify!($e))
    }};
}

fn system_now() -> String {
    chrono::Local::now().to_rfc3339()
}

//
// Relevant code starts here!
//

struct FileIoManager {
    files: std::sync::Mutex<Vec<(String, Vec<u8>)>>,
}
impl FileIoManager {
    fn new() -> Self {
        Self {
            files: std::sync::Mutex::new(Vec::new()),
        }
    }
    fn add_file(&self, name: &str, bytes: Vec<u8>) {
        let mut files = log_result!(self.files.lock());
        files.push((name.to_string(), bytes));
    }
    fn extract_files(&self) -> Option<Vec<(String, Vec<u8>)>> {
        let mut files = log_result!(self.files.lock());
        if files.is_empty() {
            None
        } else {
            let mut extracted_files = Vec::new();
            std::mem::swap(files.as_mut(), &mut extracted_files);
            assert!(files.is_empty());
            Some(extracted_files)
        }
    }
}

const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

struct SurfaceState {
    window: Arc<Window>,
    surface: Surface<'static>,
    size: PhysicalSize<u32>,
}
impl SurfaceState {
    fn new(instance: &Instance, window: Arc<Window>) -> Self {
        let surface = log_result!(instance.create_surface(window.clone()));
        Self::from_existing(window, surface)
    }
    fn from_existing(window: Arc<Window>, surface: Surface<'static>) -> Self {
        Self {
            window,
            surface,
            size: PhysicalSize::new(0, 0),
        }
    }
    fn configure(&mut self, device: &Device) -> bool {
        let size = self.window.inner_size();
        let is_ready = size.width > 0 && size.height > 0;
        if is_ready && self.size != size {
            self.surface.configure(
                device,
                &SurfaceConfiguration {
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    format: SWAPCHAIN_FORMAT,
                    width: size.width,
                    height: size.height,
                    present_mode: PresentMode::AutoVsync,
                    desired_maximum_frame_latency: 2,
                    alpha_mode: CompositeAlphaMode::Auto,
                    view_formats: Vec::new(),
                },
            );
        }
        self.size = size;
        is_ready
    }
    fn current_texture(&mut self, device: &Device) -> Option<SurfaceTexture> {
        if self.configure(device) {
            match self.surface.get_current_texture() {
                Ok(surface_texture) => Some(surface_texture),
                Err(SurfaceError::OutOfMemory) => panic!("SurfaceError::OutOfMemory"),
                _ => {
                    self.size = PhysicalSize::new(0, 0);
                    None
                }
            }
        } else {
            None
        }
    }
}

struct GpuState {
    instance: Instance,
    device: Device,
    queue: Queue,
}
impl GpuState {
    fn instance() -> Instance {
        Instance::new(InstanceDescriptor {
            backends: Backends::PRIMARY,
            flags: InstanceFlags::debugging(),
            dx12_shader_compiler: Dx12Compiler::default(),
            gles_minor_version: Gles3MinorVersion::default(),
        })
    }

    #[cfg(not(target_family = "wasm"))]
    fn from_window(window: Arc<Window>) -> (Self, SurfaceState) {
        let instance = Self::instance();
        let surface = log_result!(instance.create_surface(window.clone()));
        let adapter = log_result!(pollster::block_on(instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }
        )));
        let (device, queue) = log_result!(pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::default(),
                required_limits: Limits::default(),
            },
            None
        )));
        let gpu_state = Self {
            instance,
            device,
            queue,
        };
        (gpu_state, SurfaceState::from_existing(window, surface))
    }

    #[cfg(target_family = "wasm")]
    async fn from_wasm() -> Self {
        let instance = Self::instance();
        let adapter = log_result!(
            instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
        );
        let (device, queue) = log_result!(
            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: None,
                        required_features: Features::default(),
                        required_limits: Limits::default(),
                    },
                    None
                )
                .await
        );
        Self {
            instance,
            device,
            queue,
        }
    }

    fn create_surface(&self, window: Arc<Window>) -> SurfaceState {
        SurfaceState::new(&self.instance, window)
    }
}

pub struct EguiState {
    pub context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiState {
    pub fn new(device: &Device, window: &Window) -> Self {
        use egui::*;
        use egui_wgpu::*;
        use egui_winit::*;
        let context = Context::default();
        let viewport_id = ViewportId::ROOT;
        let native_pixels_per_point = Some(window.scale_factor() as f32);
        let max_texture_side = device.limits().max_texture_dimension_2d.min(2048);
        let max_texture_side = Some(max_texture_side as usize);
        let state = State::new(
            context.clone(),
            viewport_id,
            &window,
            native_pixels_per_point,
            max_texture_side,
        );
        let renderer = Renderer::new(device, SWAPCHAIN_FORMAT, None, 1);

        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        run_ui: impl FnOnce(&egui::Context),
    ) {
        let raw_input = self.state.take_egui_input(&window);
        let full_output = self.context.run(raw_input, |ui| {
            run_ui(ui);
        });
        self.state
            .handle_platform_output(window, full_output.platform_output);
        let pixels_per_point = window.scale_factor() as f32;
        let tris = self
            .context
            .tessellate(full_output.shapes, pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer
            .update_buffers(&device, &queue, encoder, &tris, &screen_descriptor);
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui RenderPass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}

struct UiState {
    num_clicks: usize,
    checked: bool,
    num_checks: usize,
    dropped_files: Vec<(String, Vec<u8>, usize)>,
}
impl UiState {
    fn new() -> Self {
        Self {
            num_clicks: 0,
            checked: false,
            num_checks: 0,
            dropped_files: Vec::new(),
        }
    }
    fn run_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Test egui window")
            .resizable([true, true])
            .show(ctx, |ui| {
                let button_text = match self.num_clicks {
                    0 => "I dare you! I double-dare you!".to_string(),
                    1 => "Oo-ooh! Now you've done it!".to_string(),
                    2 => "Oo-ooh! Now you've done it! Twice! >:[".to_string(),
                    _ => format!(
                        "Oo-ooh! Now you've done it! {} times already... m(_ _)m",
                        self.num_clicks
                    ),
                };
                if ui.button(button_text).clicked() {
                    self.num_clicks += 1;
                }
                if self.num_clicks > 0 {
                    ui.label(format!(
                        "You've clicked the button {} time(s)",
                        self.num_clicks
                    ));
                }
                if ui.checkbox(&mut self.checked, "Some checkbox").changed() {
                    if self.checked {
                        self.num_checks += 1;
                    }
                }
                let label_text = if self.checked {
                    "The checkbox *is* checked"
                } else {
                    "The checkbox is *not* checked"
                };
                ui.label(label_text);
                ui.label(format!(
                    "The checkbox has been checked {} time(s)",
                    self.num_checks
                ));

                if !self.dropped_files.is_empty() {
                    egui::Grid::new("dropped files").show(ui, |ui| {
                        ui.label("filename");
                        ui.label("size (bytes)");
                        ui.end_row();
                        for (name, _bytes, sum) in &self.dropped_files {
                            ui.label(name.as_str());
                            ui.label(format!("{}", *sum));
                            ui.end_row();
                        }
                    });
                }
            });
    }
    fn drop_file(&mut self, name: String, bytes: Vec<u8>) {
        let sum: usize = bytes.iter().map(|b| *b as usize).sum();
        self.dropped_files.push((name, bytes, sum));
    }
}

struct App {
    file_io_manager: Arc<FileIoManager>,
    window: Option<Arc<winit::window::Window>>,
    surface: Option<SurfaceState>,
    gpu_state: Option<GpuState>,
    egui_state: Option<EguiState>,
    ui_state: UiState,
    start_millis: i64,
}
impl App {
    #[cfg(target_family = "wasm")]
    fn create_window(event_loop: &ActiveEventLoop) -> Window {
        use wasm_bindgen::prelude::*;
        let window = log_result!(web_sys::window());
        let document = log_result!(window.document());
        let canvas = log_result!(document.get_element_by_id("rust_canvas"));
        let canvas: web_sys::HtmlCanvasElement =
            log_result!(canvas.dyn_into::<web_sys::HtmlCanvasElement>());
        use winit::platform::web::WindowAttributesExtWebSys;
        log_result!(event_loop.create_window(Window::default_attributes().with_canvas(Some(canvas))))
    }

    #[cfg(not(target_family = "wasm"))]
    fn create_window(event_loop: &ActiveEventLoop) -> Window {
        log_result!(event_loop.create_window(Window::default_attributes()))
    }

    fn new() -> Self {
        Self {
            file_io_manager: Arc::new(FileIoManager::new()),
            window: None,
            surface: None,
            gpu_state: None,
            egui_state: None,
            ui_state: UiState::new(),
            start_millis: chrono::Local::now().timestamp_millis(),
        }
    }

    fn clone_file_io_manager(&self) -> Arc<FileIoManager> {
        self.file_io_manager.clone()
    }

    fn current_color(&self) -> Color {
        use palette::{FromColor, Hsl, Srgb};
        let millis = (chrono::Local::now().timestamp_millis() - self.start_millis).abs();
        let t = (millis % 5000) as f64 / 5000.0;
        let hue = (360.0 * t) as f32;
        let hsl = Hsl::new(hue, 0.5, 0.5);
        let rgb: Srgb = Srgb::from_color(hsl);
        Color {
            r: rgb.red as f64,
            g: rgb.green as f64,
            b: rgb.blue as f64,
            a: 1.0,
        }
    }

    #[cfg(target_family = "wasm")]
    async fn init_wasm_gpu(&mut self) {
        self.gpu_state = Some(GpuState::from_wasm().await)
    }

    #[cfg(not(target_family = "wasm"))]
    fn resumed_impl(&mut self, event_loop: &ActiveEventLoop) {
        let window = self
            .window
            .get_or_insert_with(|| Arc::new(Self::create_window(event_loop)))
            .clone();
        if let Some(gpu_state) = &self.gpu_state {
            if self.surface.is_none() {
                self.surface = Some(gpu_state.create_surface(window));
            }
        } else {
            let (gpu_state, surface) = GpuState::from_window(window);
            self.gpu_state = Some(gpu_state);
            self.surface = Some(surface);
        }
    }
    #[cfg(target_family = "wasm")]
    fn resumed_impl(&mut self, event_loop: &ActiveEventLoop) {
        let window = self
            .window
            .get_or_insert_with(|| Arc::new(Self::create_window(event_loop)))
            .clone();
        assert!(self.gpu_state.is_some());
        if let Some(gpu_state) = &self.gpu_state {
            if self.surface.is_none() {
                self.surface = Some(gpu_state.create_surface(window));
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // This method is called eg. when the application starts, when the user
        // browses 'back' to the webpage, when the OS resumes the application...
        log::info!("ApplicationHandler::resumed() for App");
        self.resumed_impl(event_loop);
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // This method is called eg. when the user browses away from the
        // webpage, when the OS suspends the application...
        log::info!("ApplicationHandler::suspended() for App");
        self.window = None;
        self.surface = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let (Some(egui_state), Some(window)) = (&mut self.egui_state, &self.window) {
            egui_state.handle_input(&window, &event);
        }
        use WindowEvent as WE;
        match event {
            WE::CloseRequested => {
                log::debug!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WE::Destroyed => {
                log::debug!("WindowEvent::Destroyed");
            }
            WE::DroppedFile(path) => {
                let bytes = log_result!(std::fs::read(&path));
                let name = log_result!(path.into_os_string().into_string());
                on_file_drop(&bytes);
                self.ui_state.drop_file(name, bytes);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WE::RedrawRequested => {
                if let (Some(window), Some(surface_state), Some(gpu_state)) =
                    (&self.window, &mut self.surface, &self.gpu_state)
                {
                    if let Some(surface_texture) = surface_state.current_texture(&gpu_state.device)
                    {
                        let view = surface_texture.texture.create_view(&TextureViewDescriptor {
                            label: None,
                            format: Some(SWAPCHAIN_FORMAT),
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: Some(1),
                            base_array_layer: 0,
                            array_layer_count: Some(1),
                        });
                        let mut encoder = gpu_state
                            .device
                            .create_command_encoder(&CommandEncoderDescriptor { label: None });
                        {
                            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                                label: None,
                                color_attachments: &[Some(RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: Operations {
                                        load: LoadOp::Clear(self.current_color()),
                                        store: StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                        }

                        let egui_state = self
                            .egui_state
                            .get_or_insert_with(|| EguiState::new(&gpu_state.device, &window));
                        let size = window.inner_size();
                        let screen_descriptor = egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [size.width, size.height],
                            pixels_per_point: window.scale_factor() as f32,
                        };
                        egui_state.draw(
                            &gpu_state.device,
                            &gpu_state.queue,
                            &mut encoder,
                            &window,
                            &view,
                            screen_descriptor,
                            |ctx| self.ui_state.run_egui(ctx),
                        );

                        let command_buffer = encoder.finish();
                        gpu_state.queue.submit(std::iter::once(command_buffer));
                        drop(view);
                        surface_texture.present();
                    }
                    window.request_redraw();
                }
            }
            WE::Resized(client_area) => {
                log::debug!(
                    "WindowEvent::Resized : width = {}, height = {}",
                    client_area.width,
                    client_area.height
                );
            }
            _ => (),
        }
    }
}

#[cfg(target_family = "wasm")]
pub mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::*;

    #[wasm_bindgen]
    struct FileIoJsContext {
        mgr: Arc<FileIoManager>,
    }
    impl FileIoJsContext {
        #[wasm_bindgen]
        pub fn duplicate(&self) -> Self {
            Self {
                mgr: self.mgr.clone(),
            }
        }
    }

    #[wasm_bindgen(start)]
    pub async fn wasm_main() -> Result<FileIoJsContext, JsValue> {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug)
            .expect("console_log::init_with_level() failed");
        log::info!("entering wasm_main() at {}...", system_now());
        let event_loop = log_result!(EventLoop::new());
        event_loop.set_control_flow(ControlFlow::Wait);
        use winit::platform::web::EventLoopExtWebSys;
        let mut app = App::new();
        let mgr = app.clone_file_io_manager();
        app.init_wasm_gpu().await;
        event_loop.spawn_app(app);
        log::info!("...exiting wasm_main() at {}", system_now());
        Ok(FileIoJsContext { mgr })
    }

    #[wasm_bindgen]
    pub fn on_file_drop(name: &str, bytes: &[u8]) {
        super::on_file_drop(bytes);
    }
}

pub fn on_file_drop(bytes: &[u8]) {
    let num_bytes = bytes.len();
    let sum: usize = bytes.iter().map(|b| *b as usize).sum();
    log::info!("on_file_drop(bytes) - bytes.len() = {num_bytes} - sum of all bytes: {sum}");
}

#[cfg(not(target_family = "wasm"))]
pub mod not_wasm {
    use super::*;
    pub fn desktop_main() -> Result<(), Box<dyn std::error::Error>> {
        env_logger::init();
        log::info!("entering desktop_main() at {}...", system_now());
        let event_loop = log_result!(EventLoop::new());
        event_loop.set_control_flow(ControlFlow::Wait);
        let mut app = App::new();
        log_result!(event_loop.run_app(&mut app));
        log::info!("...exiting desktop_main() at {}", system_now());
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    not_wasm::desktop_main()
}
