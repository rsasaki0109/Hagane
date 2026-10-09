//! Native import-only timing; model construction and verification are untimed.
use hagane::*;
use std::{hint::black_box, time::Instant};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("expected one optional repetition count (1..100)".into());
    }
    let repetitions: usize = args.first().map(|v| v.parse()).transpose()?.unwrap_or(3);
    if !(1..=100).contains(&repetitions) {
        return Err("repetitions must be in 1..100".into());
    }
    let tolerance = Tolerance::default();
    let dimensions = [80., 60., 20.];
    let hole = [[0.35, 0.65], [0.35, 0.65]];
    let mut fixtures = Vec::new();
    for bulge in [-12., 0.001, 30., 36.] {
        let source = NurbsGraphSolid::new(dimensions, bulge, tolerance)?;
        let part = NurbsGraphHoledSolid::new(&source, hole, tolerance)?;
        fixtures.push((bulge, part.export_step_mm(tolerance)?));
    }
    let mut results = Vec::new();
    for (bulge, step) in fixtures {
        let mut samples_ms = Vec::new();
        for _ in 0..repetitions {
            let start = Instant::now();
            let imported = import_step_nurbs_graph_holed_mm(black_box(&step), tolerance);
            let elapsed = start.elapsed();
            let imported = imported?;
            imported.validate(tolerance)?;
            if imported.export_step_mm(tolerance)? != step {
                return Err("import changed the canonical exact B-rep STEP representation".into());
            }
            samples_ms.push(elapsed.as_secs_f64() * 1000.);
        }
        let mut sorted = samples_ms.clone();
        sorted.sort_by(f64::total_cmp);
        results.push(serde_json::json!({
            "bulge":bulge,"dimensions":dimensions,"hole":hole,"step_bytes":step.len(),
            "samples_ms":samples_ms,"minimum_ms":sorted[0],
            "median_ms":sorted[sorted.len()/2],"validated":true,"exact_step_identity":true
        }));
    }
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "benchmark":"nurbs_graph_holed_step_import","timed_operation":"import_step_nurbs_graph_holed_mm",
            "profile":if cfg!(debug_assertions){"debug"}else{"release"},
            "architecture":std::env::consts::ARCH,"os":std::env::consts::OS,
            "pointer_bits":usize::BITS,"crate_version":env!("CARGO_PKG_VERSION"),
            "rustc":String::from_utf8(rustc.stdout)?.trim(),"repetitions":repetitions,
            "construction_timed":false,"verification_timed":false,"display_meshing":false,
            "fixtures":results
        }))?
    );
    Ok(())
}
