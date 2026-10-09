use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut x = [
        80., 60., 20., 30., 0.2, 0., 0., 0., 0., 0., 1., 0., 1., 0.35, 0.65, 0.35, 0.65, 20., 15.,
        10., 1e-8,
    ];
    for (i, v) in x.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *v = arg.parse()?;
        }
    }
    let mode = args
        .get(21)
        .map(|v| v.parse::<u32>())
        .transpose()?
        .unwrap_or(1);
    if mode > 1 {
        return Err("graph classification mode must be 0 or 1".into());
    }
    let result = if mode == 0 {
        nurbs_graph_point_demo_json(
            x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8], x[9], x[10], x[11], x[12], x[17],
            x[18], x[19], x[20],
        )
    } else {
        nurbs_graph_hole_point_demo_json(
            x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8], x[9], x[10], x[11], x[12], x[13],
            x[14], x[15], x[16], x[17], x[18], x[19], x[20],
        )
    };
    println!("{}", result?);
    Ok(())
}
