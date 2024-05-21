# wasm_winit_wgpu

Prerequisites:
```
> cargo install wasm-pack
> cargo install miniserve
```

Run as a desktop app:
```
> cargo run --bin desktop
```

To show logging output on `windows` use:
```
> set RUST_LOG=wasm_winit_wgpu
```

Build as `wasm` and host using miniserve:
```
> wasm-pack build --target web
> miniserve . --index "index.html" -p 8080
```

Build as `wasm` and host using python(3):
```
> wasm-pack build --target web
> python3 -m http.server 8080
```

Useful links:

https://rustwasm.github.io/wasm-bindgen/examples/without-a-bundler.html
https://github.com/rust-windowing/winit/issues/3560