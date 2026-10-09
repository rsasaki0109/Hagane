fn main() {
    let offset = std::env::args()
        .nth(1)
        .map(|v| v.parse().expect("numeric offset"))
        .unwrap_or(0.);
    println!(
        "{}",
        hagane::solid_split_components_demo_json(offset).unwrap()
    );
}
