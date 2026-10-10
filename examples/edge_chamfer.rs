use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional ten-number JSON payload".into());
    }
    let x: Vec<f64> = if let Some(text) = args.first() {
        serde_json::from_str(text)?
    } else {
        vec![80., 60., 20., 0., 3., 0., 0., 0., 0., 1e-8]
    };
    println!("{}", edge_chamfer_demo_json(&x)?);
    Ok(())
}
