fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .map(|v| v.parse().expect("mode must be 0 or 1"))
        .unwrap_or(0);
    let offset = args
        .next()
        .map(|v| v.parse().expect("offset must be numeric"))
        .unwrap_or(0.);
    println!(
        "{}",
        hagane::planar_convex_boolean_demo_json(mode, offset).unwrap()
    );
}
