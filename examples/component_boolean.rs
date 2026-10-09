fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .map(|v| v.parse().expect("mode 0 or 1"))
        .unwrap_or(0);
    let offset = args
        .next()
        .map(|v| v.parse().expect("numeric offset"))
        .unwrap_or(0.);
    println!(
        "{}",
        hagane::component_boolean_demo_json(mode, offset).unwrap()
    );
}
