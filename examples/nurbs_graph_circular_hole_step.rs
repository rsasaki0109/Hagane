use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional 16-number JSON payload".into());
    }
    let values: Vec<f64> = if let Some(s) = args.first() {
        serde_json::from_str(s)?
    } else {
        vec![
            80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1., 40., 30., 12.,
        ]
    };
    println!("{}", nurbs_graph_circular_hole_step_demo_json(&values)?);
    Ok(())
}
