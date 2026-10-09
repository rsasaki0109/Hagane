use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 15 {
        return Err(
            "expected model13, optional outer polygon JSON/null, optional opening JSON".into(),
        );
    }
    let mut x = vec![80., 60., 20., 30., 0.5, 0., 0., 0., 0., 0., 1., 0., 1.];
    for (i, v) in x.iter_mut().enumerate() {
        if let Some(s) = args.get(i) {
            *v = s.parse()?;
        }
    }
    let outer: Option<Vec<[f64; 2]>> = if let Some(s) = args.get(13) {
        serde_json::from_str(s)?
    } else {
        None
    };
    if outer.as_ref().is_some_and(|p| !(3..=16).contains(&p.len())) {
        return Err("outer polygon requires 3..16 UV pairs or null".into());
    }
    let opening: Vec<[f64; 2]> = if let Some(s) = args.get(14) {
        serde_json::from_str(s)?
    } else {
        vec![[0.3, 0.5], [0.5, 0.3], [0.7, 0.5], [0.5, 0.7]]
    };
    let outer = outer.unwrap_or_default();
    x.extend([outer.len() as f64, opening.len() as f64]);
    x.extend(outer.into_iter().flatten());
    x.extend(opening.into_iter().flatten());
    println!("{}", nurbs_graph_polygon_hole_demo_json(&x)?);
    Ok(())
}
