use hagane::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut input = [80., 60., 20., 30., 0.2];
    for (i, value) in input.iter_mut().enumerate() {
        if let Some(arg) = args.get(i) {
            *value = arg.parse()?;
        }
    }
    println!(
        "{}",
        nurbs_graph_solid_demo_json(input[0], input[1], input[2], input[3], input[4])?
    );
    Ok(())
}
