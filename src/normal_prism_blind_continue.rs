//! Additional disjoint blind pockets while retaining actual previously machined geometry.
use crate::*;

const DOMAIN: Error = Error::Unsupported("continued blind bore graft or arithmetic is unresolved");
fn normalized_axis(axis: Vec3) -> Result<Vec3> {
    let scale = axis.x.abs().max(axis.y.abs()).max(axis.z.abs());
    if !axis.finite() || !scale.is_finite() || scale == 0. {
        return Err(Error::InvalidInput(
            "blind bore axis must be finite and nonzero",
        ));
    }
    Vec3::new(axis.x / scale, axis.y / scale, axis.z / scale).normalized()
}
fn add(sum: &mut f64, correction: &mut f64, value: f64) -> Result<()> {
    let adjusted = value - *correction;
    let next = *sum + adjusted;
    *correction = (next - *sum) - adjusted;
    *sum = next;
    if sum.is_finite() && correction.is_finite() {
        Ok(())
    } else {
        Err(DOMAIN)
    }
}

/// Append disjoint blind pockets to uniform normal stock or a certified body
/// containing quarter-arc blind pockets. Previously machined or imported
/// geometry is retained unchanged; only new cavity boundaries are appended.
/// The returned removed tools, face indices and analytic volume describe only
/// the new inputs, in their input order. Combined projected disks must remain
/// strictly disjoint, including pockets on opposite entry sides. At most 16
/// pockets are admitted, subject to the original profile/opening resource limits.
pub fn append_blind_bores_normal_prism(
    source: &Solid,
    specs: &[NormalPrismBlindBoreSpec],
    extrusion_axis: Vec3,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismBlindBores> {
    if !(1..=16).contains(&specs.len()) {
        return Err(Error::InvalidInput(
            "continued blind bores require 1 through 16 new pockets",
        ));
    }
    let axis = normalized_axis(extrusion_axis)?;
    let tol = tolerance.absolute();
    source.validate(tol)?;
    match crate::arc_line_prism_validation::recognize_validated_normal_prism(source, axis, tol) {
        Ok(_) => return blind_bores_normal_prism(source, specs, axis, tolerance),
        Err(Error::Unsupported(_)) => {}
        Err(error) => return Err(error),
    }
    let recovered =
        crate::normal_prism_blind_validation::recover_validated_normal_prism_blind(source, tol)?;
    crate::arc_line_prism_validation::recognize_validated_normal_prism(
        &recovered.stock,
        axis,
        tol,
    )?;
    let previous = recovered.specs.len();
    if previous + specs.len() > 16 {
        return Err(Error::Unsupported(
            "continued blind bore combined pocket limit exceeded",
        ));
    }
    let mut combined = recovered.specs;
    if recovered.axis.dot(axis) < 0. {
        for spec in &mut combined {
            spec.entry = match spec.entry {
                NormalPrismBoreEntry::Positive => NormalPrismBoreEntry::Negative,
                NormalPrismBoreEntry::Negative => NormalPrismBoreEntry::Positive,
            };
        }
    }
    combined.extend_from_slice(specs);
    // This applies the caller's full absolute/relative policy to BOTH old and
    // new pockets, not merely the recovery certificate's absolute reservation.
    let witness = blind_bores_normal_prism(&recovered.stock, &combined, axis, tolerance)?;
    let body = witness.kept();
    let first_vertex = recovered.stock.vertices.len() + 8 * previous;
    let first_edge = recovered.stock.edges.len() + 12 * previous;
    let first_face = recovered.stock.shell.faces.len() + 5 * previous;
    if body.vertices.len() != first_vertex + 8 * specs.len()
        || body.edges.len() != first_edge + 12 * specs.len()
        || body.shell.faces.len() != first_face + 5 * specs.len()
        || recovered.stock_face_to_input.len() != recovered.stock.shell.faces.len()
    {
        return Err(DOMAIN);
    }
    let mut kept = source.clone();
    let vertex_base = kept.vertices.len();
    let edge_base = kept.edges.len();
    let face_base = kept.shell.faces.len();
    kept.vertices
        .extend(body.vertices[first_vertex..].iter().cloned());
    for edge in &body.edges[first_edge..] {
        if edge
            .vertices
            .iter()
            .any(|v| *v < first_vertex || *v >= body.vertices.len())
        {
            return Err(DOMAIN);
        }
        let mut edge = edge.clone();
        edge.vertices = edge.vertices.map(|v| vertex_base + v - first_vertex);
        kept.edges.push(edge);
    }
    let mut mouth_count = vec![0usize; specs.len()];
    for (stock_face, input_face) in recovered.stock_face_to_input.iter().enumerate() {
        if *input_face >= source.shell.faces.len() {
            return Err(DOMAIN);
        }
        for wire in &body.shell.faces[stock_face].wires {
            if !wire.coedges.iter().any(|c| c.edge >= first_edge) {
                continue;
            }
            if wire.coedges.len() != 4
                || wire
                    .coedges
                    .iter()
                    .any(|c| c.edge < first_edge || c.edge >= body.edges.len())
            {
                return Err(DOMAIN);
            }
            let pocket = (wire.coedges[0].edge - first_edge) / 12;
            if wire
                .coedges
                .iter()
                .any(|c| (c.edge - first_edge) / 12 != pocket)
            {
                return Err(DOMAIN);
            }
            mouth_count[pocket] += 1;
            let mut wire = wire.clone();
            for c in &mut wire.coedges {
                c.edge = edge_base + c.edge - first_edge;
            }
            kept.shell.faces[*input_face].wires.push(wire);
        }
    }
    if mouth_count.iter().any(|count| *count != 1) {
        return Err(DOMAIN);
    }
    for face in &body.shell.faces[first_face..] {
        let mut face = face.clone();
        for wire in &mut face.wires {
            for c in &mut wire.coedges {
                if c.edge < first_edge || c.edge >= body.edges.len() {
                    return Err(DOMAIN);
                }
                c.edge = edge_base + c.edge - first_edge;
            }
        }
        kept.shell.faces.push(face);
    }
    let mut holes = Vec::with_capacity(specs.len());
    let mut floors = Vec::with_capacity(specs.len());
    for (ids, floor) in witness.hole_faces()[previous..]
        .iter()
        .zip(&witness.floor_faces()[previous..])
    {
        if ids
            .iter()
            .chain(std::iter::once(floor))
            .any(|id| *id < first_face || *id >= body.shell.faces.len())
        {
            return Err(DOMAIN);
        }
        holes.push(ids.map(|id| face_base + id - first_face));
        floors.push(face_base + floor - first_face);
    }
    let mut removed = Vec::with_capacity(specs.len());
    let mut direct = 0.;
    let mut correction = 0.;
    let mut measured = 0.;
    let mut measured_correction = 0.;
    for (spec, tool) in specs.iter().zip(&witness.removed()[previous..]) {
        tool.validate(tol)?;
        let volume = tool.volume()?;
        let analytic = std::f64::consts::PI * spec.radius * spec.radius * spec.depth;
        if !volume.is_finite()
            || volume <= 0.
            || !analytic.is_finite()
            || analytic < f64::MIN_POSITIVE
            || (volume - analytic).abs() > 8192. * f64::EPSILON * volume.max(analytic)
        {
            return Err(DOMAIN);
        }
        add(&mut direct, &mut correction, analytic)?;
        add(&mut measured, &mut measured_correction, volume)?;
        removed.push(tool.clone());
    }
    kept.validate(tol)?;
    let before = source.volume()?;
    let after = kept.volume()?;
    let budget = 8192. * f64::EPSILON * before.abs().max(after.abs()).max(measured);
    if !before.is_finite()
        || before <= 0.
        || !after.is_finite()
        || after <= 0.
        || !budget.is_finite()
        || (before - after - measured).abs() > budget
        || (direct - measured).abs() > 8192. * f64::EPSILON * direct.max(measured)
    {
        return Err(DOMAIN);
    }
    Ok(NormalPrismBlindBores::from_parts(
        kept, removed, holes, floors, direct,
    ))
}
