use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional model/count/edge-radius numeric JSON payload".into());
    }
    let x: Vec<f64> = if let Some(text) = args.first() {
        serde_json::from_str(text)?
    } else {
        vec![
            80., 60., 20., 0., 0., 0., 0., 1e-6, 0.1, 4., 8., 3., 9., 3., 10., 3., 11., 3.,
        ]
    };
    println!("{}", edge_fillet_demo_json(&x)?);
    Ok(())
}
