//! Minimal WASM C ABI. Returned bytes live until the next `hagane_generate` call.
//! No bindings, native geometry libraries, or alternate browser geometry path.
#[cfg(target_arch = "wasm32")]
mod exports {
    use std::sync::Mutex;
    static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    #[no_mangle]
    pub extern "C" fn hagane_generate(radius: f64, chord_error: f64) -> i32 {
        let (status, text) = match crate::demo_json(radius, chord_error) {
            Ok(s) => (0, s),
            Err(e) => (1, format!("{{\"error\":\"{e}\"}}")),
        };
        *OUTPUT.lock().expect("output mutex") = text.into_bytes();
        status
    }
    #[no_mangle]
    pub extern "C" fn hagane_output_ptr() -> *const u8 {
        OUTPUT.lock().expect("output mutex").as_ptr()
    }
    #[no_mangle]
    pub extern "C" fn hagane_output_len() -> usize {
        OUTPUT.lock().expect("output mutex").len()
    }
}
