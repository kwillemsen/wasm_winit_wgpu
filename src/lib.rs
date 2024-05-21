use std::sync::Arc;
use wgpu::{core::instance, *};
use winit::{application::*, event::*, event_loop::*, window::*};

//
// Irrelevant utility shizzle
//

mod fallible_stmt_detail {
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
}

macro_rules! log_result {
    ($e:expr) => {{
        use $crate::fallible_stmt_detail::LogResult;
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
    device: Device,
    queue: Queue,
}
impl GpuState {
    #[cfg(not(target_family = "wasm"))]
    fn from_window(_window: Arc<Window>) -> (Self, Surface<'static>) {
        todo!()
    }

    #[cfg(target_family = "wasm")]
    async fn from_wasm() -> Self {
        todo!()
    }

    fn create_surface(&self, window: Arc<Window>) -> Surface<'static> {
        todo!()
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

    fn instance() -> Instance {
        Instance::new(InstanceDescriptor {
        backends: Backends::PRIMARY,
        flags: InstanceFlags::debugging(),
        dx12_shader_compiler: Dx12Compiler::default(),
        gles_minor_version: Gles3MinorVersion::default(),
    })
    }

    fn new() -> Self {
        Self::default()
    }

    async fn init_wasm_gpu(&mut self) {
        todo!()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // This method is called eg. when the application starts, when the user
        // browses 'back' to the webpage, when the OS resumes the application...
        log::info!("ApplicationHandler::resumed() for App");
        let window = self.window.get_or_insert_with(|| Arc::new(Self::create_window(event_loop))).clone();
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
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // This method is called eg. when the user browses away from the
        // webpage, when the OS suspends the application...
        log::info!("ApplicationHandler::suspended() for App");
        self.window = None;
        self.surface = None;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(client_area) => {
                log::info!(
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
        app.init_wasm_gpu();
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
