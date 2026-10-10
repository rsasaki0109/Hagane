//! Minimal WASM C ABI. Returned bytes live until the next `hagane_generate*` call.
//! No bindings, native geometry libraries, or alternate browser geometry path.
#[cfg(target_arch = "wasm32")]
mod exports {
    use std::sync::Mutex;
    static EDGE_CHAMFER_MULTI_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static EDGE_CHAMFER_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_WORKFLOW_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_WORKFLOW_SESSION: Mutex<Option<crate::GraphWorkflowSession>> = Mutex::new(None);
    static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static GRAPH_CIRCULAR_HOLE_POINT_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_CIRCULAR_HOLE_STEP_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_CIRCULAR_HOLE_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_RATIONAL_ROOF_CIRCLE_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_STEP_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_HOLE_STEP_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_POINT_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_HOLE_POINT_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_HOLE_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_MULTI_HOLE_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_MULTI_HOLE_STEP_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_MULTI_HOLE_POINT_INPUT: Mutex<(Vec<f64>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_SPLIT_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_INPUT: Mutex<(Vec<f64>, bool)> = Mutex::new((Vec::new(), false));
    static WORKFLOW_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    static WORKFLOW_SESSION: Mutex<Option<crate::WorkflowSession>> = Mutex::new(None);
    static GRAPH_HOLE_STEP_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_AUTO_STEP_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    static GRAPH_POLYGON_STEP_IMPORT_INPUT: Mutex<(Vec<u8>, bool)> =
        Mutex::new((Vec::new(), false));
    static GRAPH_STEP_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    static STEP_INPUT: Mutex<(Vec<u8>, bool)> = Mutex::new((Vec::new(), false));
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_begin() {
        let mut input = GRAPH_WORKFLOW_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_push_byte(byte: u32) -> i32 {
        let mut input = GRAPH_WORKFLOW_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= 64 * 1024 {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_finish() -> i32 {
        let (bytes, bad) = {
            let mut input = GRAPH_WORKFLOW_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        let result = if bad {
            Err(crate::Error::InvalidInput(
                "graph workflow document exceeds 64 KiB or has invalid bytes",
            ))
        } else {
            match std::str::from_utf8(&bytes) {
                Ok(text) => GRAPH_WORKFLOW_SESSION
                    .lock()
                    .unwrap()
                    .get_or_insert_with(crate::GraphWorkflowSession::new)
                    .evaluate_json(text),
                Err(_) => Err(crate::Error::InvalidInput(
                    "graph workflow document must be UTF-8",
                )),
            }
        };
        graph_workflow_generate(result)
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_reset() -> i32 {
        *GRAPH_WORKFLOW_SESSION.lock().unwrap() = None;
        let mut input = GRAPH_WORKFLOW_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
        graph_workflow_generate(Ok("{\"ok\":true}".into()))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_finish_step_export() -> i32 {
        let session = GRAPH_WORKFLOW_SESSION.lock().unwrap();
        graph_workflow_generate(match session.as_ref() {
            Some(s) => crate::graph_workflow_step_export_json(s),
            None => Err(crate::Error::InvalidInput(
                "graph workflow has no accepted shape",
            )),
        })
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_workflow_finish_point_query(
        x: f64,
        y: f64,
        z: f64,
        linear: f64,
    ) -> i32 {
        let session = GRAPH_WORKFLOW_SESSION.lock().unwrap();
        graph_workflow_generate(match session.as_ref() {
            Some(s) => {
                crate::graph_workflow_point_query_json(s, crate::Point3::new(x, y, z), linear)
            }
            None => Err(crate::Error::InvalidInput(
                "graph workflow has no accepted shape",
            )),
        })
    }
    fn graph_workflow_generate(result: crate::Result<String>) -> i32 {
        let (status, text) = crate::graph_workflow_output(result);
        *OUTPUT.lock().unwrap() = text.into_bytes();
        status
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_begin() {
        let mut input = GRAPH_POLYGON_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_STEP_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 45 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "polygon STEP transport rejected input",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_step_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_step_begin() {
        let mut input = GRAPH_POLYGON_HOLE_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_step_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_HOLE_STEP_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 79 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_step_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_HOLE_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "polygon STEP transport rejected input",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_hole_step_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_point_begin() {
        let mut input = GRAPH_POLYGON_POINT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_point_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_POINT_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 49 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_point_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_POINT_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "polygon point query transport rejected input",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_point_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_point_begin() {
        let mut input = GRAPH_POLYGON_HOLE_POINT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_point_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_HOLE_POINT_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 83 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_point_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_HOLE_POINT_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "polygon point query transport rejected input",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_hole_point_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_point_begin() {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_POINT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_point_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_POINT_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 151 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_point_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_MULTI_HOLE_POINT_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "multiple polygon opening query transport requires at most 151 finite values",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_multi_hole_point_demo_json(
                &values,
            ))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_step_begin() {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_step_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_STEP_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 147 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_step_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_MULTI_HOLE_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "multiple polygon STEP transport requires at most 147 finite values",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_multi_hole_step_demo_json(
                &values,
            ))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_begin() {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_MULTI_HOLE_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 147 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_multi_hole_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_MULTI_HOLE_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "multiple polygon opening transport requires at most 147 finite values",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_multi_hole_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_begin() {
        let mut input = GRAPH_POLYGON_HOLE_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_HOLE_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 79 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_hole_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_HOLE_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "polygon opening transport requires at most 79 finite values",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_hole_demo_json(&values))
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_multi_begin() {
        let mut input = EDGE_CHAMFER_MULTI_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_multi_push(value: f64) -> i32 {
        let mut input = EDGE_CHAMFER_MULTI_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 33 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_multi_finish() -> i32 {
        let (values, bad) = {
            let mut input = EDGE_CHAMFER_MULTI_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "multi edge chamfer transport needs at most 33 finite values",
            )));
        }
        generate(crate::edge_chamfer_multi_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_begin() {
        let mut input = EDGE_CHAMFER_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_push(value: f64) -> i32 {
        let mut input = EDGE_CHAMFER_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 10 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_edge_chamfer_finish() -> i32 {
        let (values, bad) = {
            let mut input = EDGE_CHAMFER_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "edge chamfer transport needs exactly 10 finite values",
            )));
        }
        generate(crate::edge_chamfer_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_point_begin() {
        let mut input = GRAPH_CIRCULAR_HOLE_POINT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_point_push(value: f64) -> i32 {
        let mut input = GRAPH_CIRCULAR_HOLE_POINT_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 20 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_point_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_CIRCULAR_HOLE_POINT_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "circular hole query transport needs exactly 20 finite values",
            )));
        }
        generate(crate::nurbs_graph_circular_hole_point_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_step_begin() {
        let mut input = GRAPH_CIRCULAR_HOLE_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_step_push(value: f64) -> i32 {
        let mut input = GRAPH_CIRCULAR_HOLE_STEP_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 16 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_step_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_CIRCULAR_HOLE_STEP_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "circular hole STEP transport needs exactly 16 finite values",
            )));
        }
        generate(crate::nurbs_graph_circular_hole_step_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_begin() {
        let mut input = GRAPH_CIRCULAR_HOLE_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_push(value: f64) -> i32 {
        let mut input = GRAPH_CIRCULAR_HOLE_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 16 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_circular_hole_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_CIRCULAR_HOLE_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "circular hole transport needs exactly 16 finite values",
            )));
        }
        generate(crate::nurbs_graph_circular_hole_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_rational_roof_circle_begin() {
        let mut input = GRAPH_RATIONAL_ROOF_CIRCLE_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_rational_roof_circle_push(value: f64) -> i32 {
        let mut input = GRAPH_RATIONAL_ROOF_CIRCLE_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 16 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_rational_roof_circle_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_RATIONAL_ROOF_CIRCLE_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "rational roof circle transport needs exactly 16 finite values",
            )));
        }
        generate(crate::nurbs_graph_rational_roof_circle_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_begin() {
        let mut input = GRAPH_POLYGON_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 45 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "polygon graph transport needs at most 45 finite values",
            )));
        }
        generate(crate::nurbs_graph_polygon_numeric_demo_json(&values))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_import_begin() {
        let mut input = GRAPH_POLYGON_STEP_IMPORT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_import_push_byte(byte: u32) -> i32 {
        let mut input = GRAPH_POLYGON_STEP_IMPORT_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= crate::STEP_IMPORT_MAX_BYTES {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_polygon_step_import_finish(error: f64) -> i32 {
        let (bytes, bad) = {
            let mut input = GRAPH_POLYGON_STEP_IMPORT_INPUT.lock().unwrap();
            (
                std::mem::take(&mut input.0),
                std::mem::replace(&mut input.1, false),
            )
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "polygon graph STEP import transport exceeds 1 MiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::nurbs_graph_polygon_step_import_demo_json(
                text, error,
            )),
            Err(_) => generate(Err(crate::Error::InvalidInput(
                "polygon graph STEP import must be UTF-8",
            ))),
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_step_import_begin() {
        let mut input = GRAPH_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_step_import_push_byte(byte: u32) -> i32 {
        let mut input = GRAPH_STEP_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= crate::STEP_IMPORT_MAX_BYTES {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_step_import_finish(error: f64) -> i32 {
        let (bytes, bad) = {
            let mut input = GRAPH_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "graph STEP import transport exceeds 1 MiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::nurbs_graph_step_import_demo_json(text, error)),
            Err(_) => generate(Err(crate::Error::InvalidInput(
                "graph STEP import must be UTF-8",
            ))),
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_auto_step_import_begin() {
        let mut input = GRAPH_AUTO_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_auto_step_import_push_byte(byte: u32) -> i32 {
        let mut input = GRAPH_AUTO_STEP_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= crate::STEP_IMPORT_MAX_BYTES {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_auto_step_import_finish(error: f64) -> i32 {
        let (bytes, bad) = {
            let mut input = GRAPH_AUTO_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "graph auto STEP import transport exceeds 1 MiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::nurbs_graph_step_auto_import_demo_json(text, error)),
            Err(_) => generate(Err(crate::Error::InvalidInput(
                "graph auto STEP import must be UTF-8",
            ))),
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_hole_step_import_begin() {
        let mut input = GRAPH_HOLE_STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_hole_step_import_push_byte(byte: u32) -> i32 {
        let mut input = GRAPH_HOLE_STEP_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= crate::STEP_IMPORT_MAX_BYTES {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_hole_step_import_finish(error: f64) -> i32 {
        let (bytes, bad) = {
            let mut input = GRAPH_HOLE_STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "holed graph STEP import transport exceeds 1 MiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::nurbs_graph_holed_step_import_demo_json(text, error)),
            Err(_) => generate(Err(crate::Error::InvalidInput(
                "holed graph STEP import must be UTF-8",
            ))),
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_step_import_begin() {
        let mut input = STEP_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[no_mangle]
    pub extern "C" fn hagane_step_import_push_byte(byte: u32) -> i32 {
        let mut input = STEP_INPUT.lock().unwrap();
        if input.1 || byte > 255 || input.0.len() >= crate::STEP_IMPORT_MAX_BYTES {
            input.1 = true;
            return 1;
        }
        input.0.push(byte as u8);
        0
    }
    #[no_mangle]
    pub extern "C" fn hagane_step_import_finish() -> i32 {
        let (bytes, bad) = {
            let mut input = STEP_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "STEP import transport exceeds 1 MiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::import_step_json(text)),
            Err(_) => generate(Err(crate::Error::InvalidInput("STEP import must be UTF-8"))),
        }
    }
    #[no_mangle]
    pub extern "C" fn hagane_workflow_reset_session() {
        *WORKFLOW_SESSION.lock().unwrap() = None;
    }
    #[no_mangle]
    pub extern "C" fn hagane_workflow_finish_incremental() -> i32 {
        finish_workflow(true)
    }
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
        finish_workflow(false)
    }
    #[no_mangle]
    pub extern "C" fn hagane_workflow_finish_step_export() -> i32 {
        let (bytes, bad) = {
            let mut input = WORKFLOW_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), input.1)
        };
        if bad {
            return generate(Err(crate::Error::InvalidInput(
                "STEP workflow input exceeds 64 KiB or has invalid bytes",
            )));
        }
        match std::str::from_utf8(&bytes) {
            Ok(text) => generate(crate::export_workflow_step_mm_json(text)),
            Err(_) => generate(Err(crate::Error::InvalidInput(
                "STEP workflow input must be UTF-8",
            ))),
        }
    }

    fn finish_workflow(incremental: bool) -> i32 {
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
            Ok(text) if incremental => generate(
                WORKFLOW_SESSION
                    .lock()
                    .unwrap()
                    .get_or_insert_with(crate::WorkflowSession::new)
                    .evaluate_json(text),
            ),
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
    pub extern "C" fn hagane_generate_nurbs_bounded(
        weight: f64,
        parameter: f64,
        chord_error: f64,
    ) -> i32 {
        generate(crate::nurbs_tessellation_demo_json(
            weight,
            parameter,
            chord_error,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_export_graph_step(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_step_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_export_graph_hole_step(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_hole_step_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, hu0, hu1, hv0,
            hv1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_section_graph(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        u: f64,
        v: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_section_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, u, v, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_section_graph_hole(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
        u: f64,
        v: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_hole_section_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, hu0, hu1, hv0,
            hv1, u, v, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_roof_section(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        start_u: f64,
        start_v: f64,
        end_u: f64,
        end_v: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_roof_section_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, start_u,
            start_v, end_u, end_v, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_graph_hole_roof_section(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
        start_u: f64,
        start_v: f64,
        end_u: f64,
        end_v: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_hole_roof_section_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, hu0, hu1, hv0,
            hv1, start_u, start_v, end_u, end_v, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_classify_graph_hole(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
        x: f64,
        y: f64,
        z: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_hole_point_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, hu0, hu1, hv0,
            hv1, x, y, z, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_classify_graph(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        x: f64,
        y: f64,
        z: f64,
        linear: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_point_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, x, y, z, linear,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_graph_solid_hole(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_hole_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1, hu0, hu1, hv0,
            hv1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_graph_solid_split(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
        axis: u32,
        parameter: f64,
        side: u32,
    ) -> i32 {
        generate(crate::nurbs_graph_split_demo_json(
            width,
            depth,
            height,
            bulge,
            error,
            angle,
            tx,
            ty,
            tz,
            u0,
            u1,
            v0,
            v1,
            axis as usize,
            parameter,
            side,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_graph_solid_trimmed(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
        u0: f64,
        u1: f64,
        v0: f64,
        v1: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_trimmed_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz, u0, u1, v0, v1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_graph_solid_placed(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
        angle: f64,
        tx: f64,
        ty: f64,
        tz: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_placed_demo_json(
            width, depth, height, bulge, error, angle, tx, ty, tz,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_graph_solid(
        width: f64,
        depth: f64,
        height: f64,
        bulge: f64,
        error: f64,
    ) -> i32 {
        generate(crate::nurbs_graph_solid_demo_json(
            width, depth, height, bulge, error,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_polygon_hole(
        height: f64,
        weight: f64,
        error: f64,
        mode: u32,
        u0: f64,
        v0: f64,
        u1: f64,
        v1: f64,
        u2: f64,
        v2: f64,
        hu0: f64,
        hu1: f64,
        hv0: f64,
        hv1: f64,
    ) -> i32 {
        generate(crate::nurbs_polygon_hole_demo_json(
            height,
            weight,
            error,
            mode,
            vec![[u0, v0], [u1, v1], [u2, v2]],
            vec![[[hu0, hu1], [hv0, hv1]]],
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_polygon(
        height: f64,
        weight: f64,
        error: f64,
        mode: u32,
        u0: f64,
        v0: f64,
        u1: f64,
        v1: f64,
        u2: f64,
        v2: f64,
    ) -> i32 {
        generate(crate::nurbs_polygon_demo_json(
            height,
            weight,
            error,
            mode,
            vec![[u0, v0], [u1, v1], [u2, v2]],
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_edge(
        height: f64,
        weight: f64,
        u0: f64,
        v0: f64,
        u1: f64,
        v1: f64,
        chord_error: f64,
        crease: u32,
    ) -> i32 {
        if crease > 1 {
            return generate(Err(crate::Error::InvalidInput(
                "crease mode must be 0 or 1",
            )));
        }
        generate(crate::nurbs_surface_edge_demo_json(
            height,
            weight,
            [u0, v0],
            [u1, v1],
            chord_error,
            crease == 1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_singular_hole(
        height: f64,
        chord_error: f64,
        hole_width: f64,
    ) -> i32 {
        generate(crate::nurbs_surface_singular_hole_demo_json(
            height,
            chord_error,
            hole_width,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_hole(
        height: f64,
        weight: f64,
        chord_error: f64,
        hole_width: f64,
        crease: u32,
    ) -> i32 {
        if crease > 1 {
            return generate(Err(crate::Error::InvalidInput(
                "crease mode must be 0 or 1",
            )));
        }
        generate(crate::nurbs_surface_hole_demo_json(
            height,
            weight,
            chord_error,
            hole_width,
            crease == 1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_trim(
        height: f64,
        weight: f64,
        u: f64,
        v: f64,
        chord_error: f64,
        u_min: f64,
        u_max: f64,
        v_min: f64,
        v_max: f64,
        crease: u32,
    ) -> i32 {
        if crease > 1 {
            return generate(Err(crate::Error::InvalidInput(
                "crease mode must be 0 or 1",
            )));
        }
        generate(crate::nurbs_surface_trim_demo_json(
            height,
            weight,
            u,
            v,
            chord_error,
            [[u_min, u_max], [v_min, v_max]],
            crease == 1,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_crease(
        height: f64,
        weight: f64,
        u: f64,
        v: f64,
        chord_error: f64,
        side: u32,
    ) -> i32 {
        generate(crate::nurbs_surface_crease_demo_json(
            height,
            weight,
            u,
            v,
            chord_error,
            side,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_multispan(
        height: f64,
        weight: f64,
        u: f64,
        v: f64,
        chord_error: f64,
    ) -> i32 {
        generate(crate::nurbs_surface_multispan_demo_json(
            height,
            weight,
            u,
            v,
            chord_error,
        ))
    }
    #[no_mangle]
    pub extern "C" fn hagane_generate_surface_bounded(
        height: f64,
        weight: f64,
        u: f64,
        v: f64,
        chord_error: f64,
    ) -> i32 {
        generate(crate::nurbs_surface_bounded_demo_json(
            height,
            weight,
            u,
            v,
            chord_error,
        ))
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
    pub extern "C" fn hagane_split_components_demo(offset: f64) -> i32 {
        generate(crate::solid_split_components_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_component_boolean_demo(mode: u32, offset: f64) -> i32 {
        generate(crate::component_boolean_demo_json(mode, offset))
    }

    #[no_mangle]
    pub extern "C" fn hagane_convex_intersection_demo(offset: f64) -> i32 {
        generate(crate::convex_intersection_demo_json(offset))
    }
    #[no_mangle]
    pub extern "C" fn hagane_planar_convex_boolean_demo(mode: u32, offset: f64) -> i32 {
        generate(crate::planar_convex_boolean_demo_json(mode, offset))
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
    pub extern "C" fn hagane_graph_polygon_split_begin() {
        let mut input = GRAPH_POLYGON_SPLIT_INPUT.lock().unwrap();
        input.0.clear();
        input.1 = false;
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn hagane_graph_polygon_split_push(value: f64) -> i32 {
        let mut input = GRAPH_POLYGON_SPLIT_INPUT.lock().unwrap();
        if input.1 || !value.is_finite() || input.0.len() >= 50 {
            input.1 = true;
            return 1;
        }
        input.0.push(value);
        0
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn hagane_graph_polygon_split_finish() -> i32 {
        let (values, bad) = {
            let mut input = GRAPH_POLYGON_SPLIT_INPUT.lock().unwrap();
            (std::mem::take(&mut input.0), std::mem::take(&mut input.1))
        };
        if bad {
            generate(Err(crate::Error::InvalidInput(
                "line split transport rejected input",
            )))
        } else {
            generate(crate::nurbs_graph_polygon_split_demo_json(&values))
        }
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
