use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut x = [
        80., 60., 20., 30., 0.2, 0.5, 15., -10., 25., 0.2, 0.8, 0.25, 0.75,
    ];
    for (i, v) in x.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *v = arg.parse()?;
        }
    }
    println!(
        "{}",
        nurbs_graph_trimmed_demo_json(
            x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], x[8], x[9], x[10], x[11], x[12]
        )?
    );
    Ok(())
}
