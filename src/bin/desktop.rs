#[cfg(not(target_family = "wasm"))]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasm_winit_wgpu::main()
}
