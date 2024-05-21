fn system_now() -> String {
    chrono::Local::now().to_rfc3339()
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
        let window = web_sys::window().expect("web_sys::window() failed");
        log::info!("let window = web_sys::window() succeeded");
        let document = window.document().expect("window.document() failed");
        log::info!("let document = window.document() succeeded");
        let canvas = document
            .get_element_by_id("rust_canvas")
            .expect("let canvas = document.get_element_by_id(\"rust_canvas\") failed");
        log::info!("let canvas = document.get_element_by_id(\"rust_canvas\") succeeded");
        let _canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>() failed");
        log::info!("let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>() succeeded");
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
        log::info!("...exiting desktop_main() at {}", system_now());
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    not_wasm::desktop_main()
}
