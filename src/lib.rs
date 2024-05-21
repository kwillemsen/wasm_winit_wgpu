use std::sync::Arc;
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

#[derive(Default)]
struct App {
    window: Option<Arc<winit::window::Window>>,
}
impl App {
    fn new() -> Self {
        Self::default()
    }

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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("ApplicationHandler::resumed() for App");
        if self.window.is_none() {
            self.window = Some(Arc::new(Self::create_window(event_loop)));
        }
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("ApplicationHandler::suspended() for App");
        self.window = None;
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
        event_loop.spawn_app(App::new());
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
