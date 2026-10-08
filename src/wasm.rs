//! Minimal WASM C ABI. Returned bytes live until the next `hagane_generate*` call.
//! No bindings, native geometry libraries, or alternate browser geometry path.
#[cfg(target_arch = "wasm32")]
mod exports {
    use std::sync::Mutex;
    static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    #[no_mangle]
    pub extern "C" fn hagane_generate(radius: f64, chord_error: f64) -> i32 {
        generate(crate::demo_json(radius, chord_error))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_preset(preset: u32, radius: f64, chord_error: f64) -> i32 {
        generate(crate::demo_preset_json(preset, radius, chord_error))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_nurbs(weight: f64, parameter: f64) -> i32 {
        generate(crate::nurbs_demo_json(weight, parameter))
    }
    fn generate(result: crate::Result<String>) -> i32 {
        let (status, text) = match result {
            Ok(s) => (0, s),
            Err(e) => {
                let message = e
                    .to_string()
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                (1, format!("{{\"error\":\"{message}\"}}"))
            }
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
