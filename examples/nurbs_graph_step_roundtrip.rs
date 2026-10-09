use hagane::*;
fn main() -> Result<()> {
    let tol = Tolerance::default();
    let original = NurbsGraphSolid::new([80., 60., 24.], 36., tol)?;
    let step = original.export_step_mm(tol)?;
    let loaded = import_step_nurbs_graph_mm(&step, tol)?;
    loaded.validate(tol)?;
    eprintln!(
        "Imported {} vertices, {} edges, {} faces; volume {} mm³",
        loaded.brep().vertices.len(),
        loaded.brep().edges.len(),
        loaded.brep().shell.faces.len(),
        loaded.volume()?
    );
    print!("{}", loaded.export_step_mm(tol)?);
    Ok(())
}
