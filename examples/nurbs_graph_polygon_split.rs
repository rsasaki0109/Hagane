use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 19 {
        return Err("expected 18 numeric arguments and optional polygon JSON".into());
    }
    let mut x = vec![
        80., 60., 20., 30., 0.2, 0., 0., 0., 0., 0., 1., 0., 1., 0., 0.25, 1., 0.75, 0.,
    ];
    for (i, v) in x.iter_mut().enumerate() {
        if let Some(s) = args.get(i) {
            *v = s.parse()?;
        }
    }
    if let Some(text) = args.get(18) {
        let polygon: Vec<[f64; 2]> = serde_json::from_str(text)?;
        if !(3..=16).contains(&polygon.len()) {
            return Err("polygon requires 3..16 UV pairs".into());
        }
        x.extend(polygon.into_iter().flatten());
    }
    println!("{}", nurbs_graph_polygon_split_demo_json(&x)?);
    Ok(())
}
