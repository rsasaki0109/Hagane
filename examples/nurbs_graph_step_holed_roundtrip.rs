use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol)?;
    let original = NurbsGraphHoledSolid::new(&source, [[0.35, 0.65], [0.35, 0.65]], tol)?;
    let step = original.export_step_mm(tol)?;
    let loaded = import_step_nurbs_graph_holed_mm(&step, tol)?;
    loaded.validate(tol)?;
    eprintln!(
        "Imported exact holed graph: {} vertices, {} edges, {} faces, volume {} mm³",
        loaded.brep().vertices.len(),
        loaded.brep().edges.len(),
        loaded.brep().shell.faces.len(),
        loaded.volume()?
    );
    print!("{}", loaded.export_step_mm(tol)?);
    Ok(())
}
