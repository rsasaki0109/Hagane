use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut x = [80., 60., 20., 30., 0.2, 0., 0., 0., 0., 0., 1., 0., 1.];
    for (i, v) in x.iter_mut().enumerate() {
        if let Some(value) = args.get(i) {
            *v = value.parse()?;
        }
    }
    let polygon = if let Some(text) = args.get(13) {
        serde_json::from_str(text)?
    } else {
        vec![
            [0.15, 0.25],
            [0.65, 0.1],
            [0.9, 0.45],
            [0.65, 0.85],
            [0.2, 0.8],
        ]
    };
    println!(
        "{}",
        nurbs_graph_polygon_demo_json(
            x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8], x[9], x[10], x[11], x[12],
            polygon
        )?
    );
    Ok(())
}
