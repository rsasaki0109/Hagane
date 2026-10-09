use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional numeric JSON payload".into());
    }
    let values: Vec<f64> = if let Some(text) = args.first() {
        serde_json::from_str(text)?
    } else {
        vec![
            80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 0., 2., 4., 0.2, 0.5, 0.3,
            0.4, 0.4, 0.5, 0.3, 0.6, 4., 0.6, 0.5, 0.7, 0.4, 0.8, 0.5, 0.7, 0.6,
        ]
    };
    println!("{}", nurbs_graph_polygon_multi_hole_demo_json(&values)?);
    Ok(())
}
