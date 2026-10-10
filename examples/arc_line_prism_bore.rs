use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional 13-number JSON payload".into());
    }
    let x: Vec<f64> = if let Some(text) = args.first() {
        serde_json::from_str(text)?
    } else {
        vec![80., 60., 20., 8., 0., 0., 8., 0., 0., 0., 0., 1e-6, 0.1]
    };
    println!("{}", arc_line_prism_bore_demo_json(&x)?);
    Ok(())
}
