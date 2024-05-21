#[cfg(target_family = "wasm")]
pub mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn main() -> Result<(), JsValue> {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug)
            .expect("console_log::init_with_level() failed");
        log::info!("entering main()...");
        let window = web_sys::window().expect("web_sys::window() failed");
        log::info!("let window = web_sys::window() succeeded");
        let document = window.document().expect("window.document() failed");
        log::info!("let document = window.document() succeeded");
        let body = document.body().expect("document.body() failed");
        log::info!("let body = document.body() succeeded");
        // Manufacture the element we're gonna append
        let val = document.create_element("p")?;
        val.set_inner_html("Hello from Rust!");
        body.append_child(&val)?;

        let canvas = document
            .get_element_by_id("rust_canvas")
            .expect("let canvas = document.get_element_by_id(\"rust_canvas\") failed");
        log::info!("let canvas = document.get_element_by_id(\"rust_canvas\") succeeded");
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>() failed");
        log::info!("let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>() succeeded");

        log::info!("...exiting main()");
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub mod not_wasm {
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        env_logger::init();
        log::info!("entering main()...");
        log::info!("...exiting main()");
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    not_wasm::main()
}
