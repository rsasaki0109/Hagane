fn main() -> hagane::Result<()> {
    let face = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>())
        .transpose()
        .map_err(|_| hagane::Error::InvalidInput("face must be an integer"))?
        .unwrap_or(5);
    let number = |i, default| {
        std::env::args()
            .nth(i)
            .map(|s| s.parse::<f64>())
            .transpose()
            .map_err(|_| hagane::Error::InvalidInput("radius/depth must be numeric"))
            .map(|v| v.unwrap_or(default))
    };
    println!(
        "{}",
        hagane::box_face_blind_bore_demo_json(face, number(2, 7.)?, number(3, 20.)?)?
    );
    Ok(())
}
