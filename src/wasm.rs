//! Minimal WASM C ABI. Returned bytes live until the next `hagane_generate*` call.
//! No bindings, native geometry libraries, or alternate browser geometry path.
#[cfg(target_arch = "wasm32")]
mod exports {
    use std::sync::Mutex;
    static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static WORKFLOW_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    #[no_mangle]
    pub extern "C" fn hagane_workflow_begin() {
        let mut input = WORKFLOW_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_workflow_push_byte(byte: u32) -> i32 {
        let mut input = WORKFLOW_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= 65536 {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_workflow_finish() -> i32 {
        let (bytes, bad) = {
            let mut input = WORKFLOW_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(crate::workflow_input_failure(
                "Invalid byte or operation document exceeds 64 KiB.",
            ));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::evaluate_workflow_json(text)),
            Err(_) => generate(crate::workflow_input_failure(
                "Operation document must be valid UTF-8.",
            )),
        }
    }

    #[no_mangle]
    pub extern "C" fn hagane_box_face_blind_bore_demo(face: u32, radius: f64, depth: f64) -> i32 {
        generate(crate::box_face_blind_bore_demo_json(face, radius, depth))
    }
    #[no_mangle]
    pub extern "C" fn hagane_blind_bore_demo(radius: f64, depth: f64) -> i32 {
        generate(crate::blind_bore_demo_json(radius, depth))
    }
    #[no_mangle]
    pub extern "C" fn hagane_oriented_bores_demo(azimuth: f64, offset: f64) -> i32 {
        generate(crate::oriented_bores_demo_json(azimuth, offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_divergent_tilted_bores_demo(tilt: f64, offset: f64) -> i32 {
        generate(crate::divergent_tilted_bores_demo_json(tilt, offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_separated_tilted_bores_demo(offset: f64) -> i32 {
        generate(crate::separated_tilted_bores_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_tilted_bore_demo(tilt: f64, offset: f64, placement: f64) -> i32 {
        generate(crate::tilted_bore_demo_json(tilt, offset, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_ellipse_multi_hole_planar_demo(
        spread: f64,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::ellipse_multi_hole_planar_demo_json(
            spread, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_ellipse_eccentric_planar_demo(
        center_x: f64,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::ellipse_eccentric_planar_demo_json(
            center_x, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_ellipse_annulus_planar_demo(offset: f64, placement: f64) -> i32 {
        generate(crate::ellipse_annulus_planar_demo_json(offset, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_ellipse_segment_planar_demo(
        sweep: f64,
        mode: u32,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::ellipse_segment_planar_demo_json(
            sweep, mode, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_half_ellipse_planar_demo(
        mode: u32,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::half_ellipse_planar_demo_json(
            mode, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_ellipse_planar_demo(offset: f64, placement: f64) -> i32 {
        generate(crate::ellipse_planar_demo_json(offset, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_harmonic_face_intersections_demo(
        selection: u32,
        mode: u32,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::harmonic_face_intersections_demo_json(
            selection, mode, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_circular_face_intersections_demo(
        selection: u32,
        mode: u32,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::circular_face_intersections_demo_json(
            selection, mode, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_extrusion_intersections_demo(
        mode: u32,
        offset: f64,
        placement: f64,
    ) -> i32 {
        generate(crate::extrusion_intersections_demo_json(
            mode, offset, placement,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate(radius: f64, chord_error: f64) -> i32 {
        generate(crate::demo_json(radius, chord_error))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_preset(preset: u32, radius: f64, chord_error: f64) -> i32 {
        generate(crate::demo_preset_json(preset, radius, chord_error))
    }
    #[no_mangle]
    pub extern "C" fn hagane_oblique_boundary_demo(slope: f64, placement: f64) -> i32 {
        generate(crate::oblique_boundary_demo_json(slope, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_skew_face_subdivision_demo(fraction: f64, placement: f64) -> i32 {
        generate(crate::skew_face_subdivision_demo_json(fraction, placement))
    }
    #[no_mangle]
    pub extern "C" fn hagane_skew_arc_extrusion_demo(radius: f64, offset: f64, height: f64) -> i32 {
        generate(crate::skew_arc_extrusion_demo_json(radius, offset, height))
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
    pub extern "C" fn hagane_classify_curved_demo(model: u32, x: f64, y: f64, z: f64) -> i32 {
        generate(crate::curved_classification_demo_json(model, x, y, z))
    }
    #[no_mangle]
    pub extern "C" fn hagane_curved_classification_mesh(model: u32) -> i32 {
        generate(crate::curved_classification_mesh_json(model))
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
    pub extern "C" fn hagane_simplified_contact_demo(offset: f64) -> i32 {
        generate(crate::simplified_contact_demo_json(offset))
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
