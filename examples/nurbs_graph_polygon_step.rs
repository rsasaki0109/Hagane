use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 2 {
        return Err("expected mode 0/1 and optional numeric model JSON".into());
    }
    let mode: u32 = args.first().map(|s| s.parse()).transpose()?.unwrap_or(0);
    if mode > 1 {
        return Err("STEP mode must be 0 (polygon) or 1 (polygon opening)".into());
    }
    let values: Vec<f64> = if let Some(text) = args.get(1) {
        serde_json::from_str(text)?
    } else {
        let mut x = vec![80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1.];
        if mode == 0 {
            x.extend([0.15, 0.25, 0.65, 0.1, 0.9, 0.45, 0.65, 0.85, 0.2, 0.8]);
        } else {
            x.extend([0., 4., 0.3, 0.5, 0.5, 0.3, 0.7, 0.5, 0.5, 0.7]);
        }
        x
    };
    println!(
        "{}",
        if mode == 0 {
            nurbs_graph_polygon_step_demo_json(&values)?
        } else {
            nurbs_graph_polygon_hole_step_demo_json(&values)?
        }
    );
    Ok(())
}
