use std::sync::Arc;
use wgpu::*;
use winit::{application::*, event::*, event_loop::*, window::*};

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

struct GpuState {
    instance: Instance,
    adapter: Adapter,
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
    fn from_window(window: Arc<Window>) -> (Self, Surface<'static>) {
        let instance = Self::instance();
        let surface = log_result!(instance.create_surface(window));
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
            adapter,
            device,
            queue,
        };
        (gpu_state, surface)
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
            adapter,
            device,
            queue,
        }
    }

    fn create_surface(&self, window: Arc<Window>) -> Surface<'static> {
        log_result!(self.instance.create_surface(window))
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<winit::window::Window>>,
    surface: Option<Surface<'static>>,
    gpu_state: Option<GpuState>,
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
        Self::default()
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
        use WindowEvent as WE;
        match event {
            WE::CloseRequested => {
                log::debug!("WindowEvent::CloseRequested");
                event_loop.exit();
            }
            WE::Destroyed => {
                log::debug!("WindowEvent::Destroyed");
            }
            WE::RedrawRequested => {
                if let Some(window) = &self.window {
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

    #[wasm_bindgen(start)]
    pub async fn wasm_main() -> Result<(), JsValue> {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug)
            .expect("console_log::init_with_level() failed");
        log::info!("entering wasm_main() at {}...", system_now());
        let event_loop = log_result!(EventLoop::new());
        event_loop.set_control_flow(ControlFlow::Wait);
        use winit::platform::web::EventLoopExtWebSys;
        let mut app = App::new();
        app.init_wasm_gpu().await;
        event_loop.spawn_app(app);
        log::info!("...exiting wasm_main() at {}", system_now());
        Ok(())
    }
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
