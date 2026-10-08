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
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface(height: f64, weight: f64, u: f64, v: f64) -> i32 {
        generate(crate::nurbs_surface_demo_json(height, weight, u, v))
    }
    /// -1/0/+1 exact orientation; 2 indicates a nonfinite input.
    #[no_mangle]
    pub extern "C" fn hagane_orient2d(ax: f64, ay: f64, bx: f64, by: f64, cx: f64, cy: f64) -> i32 {
        crate::orient2d([ax, ay], [bx, by], [cx, cy])
            .map(|o| o.sign())
            .unwrap_or(2)
    }
    /// -1/0/+1 exact 3D orientation; 2 indicates nonfinite input.
    #[no_mangle]
    pub extern "C" fn hagane_orient3d(
        ax: f64,
        ay: f64,
        az: f64,
        bx: f64,
        by: f64,
        bz: f64,
        cx: f64,
        cy: f64,
        cz: f64,
        dx: f64,
        dy: f64,
        dz: f64,
    ) -> i32 {
        crate::orient3d([ax, ay, az], [bx, by, bz], [cx, cy, cz], [dx, dy, dz]).unwrap_or(2)
    }
    /// 0 disjoint, 1 intersecting (including contact), 2 invalid input.
    #[no_mangle]
    pub extern "C" fn hagane_segments_intersect2d(
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
        cx: f64,
        cy: f64,
        dx: f64,
        dy: f64,
    ) -> i32 {
        crate::segments_intersect2d([ax, ay], [bx, by], [cx, cy], [dx, dy])
            .map(i32::from)
            .unwrap_or(2)
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_predicates(scale: f64, angle: f64) -> i32 {
        generate(crate::predicates_demo_json(scale, angle))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_intersections(mode: u32, offset: f64, placement: f64) -> i32 {
        generate(crate::intersections_demo_json(mode, offset, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_face_clipping(offset: f64, placement: f64) -> i32 {
        generate(crate::face_clipping_demo_json(offset, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_classify_demo(x: f64, y: f64, z: f64) -> i32 {
        generate(crate::classification_demo_json(x, y, z))
    }
    #[no_mangle]
    pub extern "C" fn hagane_classification_mesh() -> i32 {
        generate(crate::classification_mesh_demo_json())
    }
    #[no_mangle]
    pub extern "C" fn hagane_partition_demo(offset: f64) -> i32 {
        generate(crate::solid_split_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_convex_intersection_demo(offset: f64) -> i32 {
        generate(crate::convex_intersection_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_reframed_merge_demo(offset: f64) -> i32 {
        generate(crate::reframed_merge_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_merged_contact_demo(offset: f64) -> i32 {
        generate(crate::merged_contact_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_box_contact_demo(offset: f64) -> i32 {
        generate(crate::box_contact_demo_json(offset))
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn hagane_convex_union_demo(offset: f64) -> i32 {
        generate(crate::convex_union_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_convex_difference_demo(offset: f64) -> i32 {
        generate(crate::convex_difference_demo_json(offset))
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
