//! Disjoint blind cavities grafted into unchanged certified normal stock.
use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct NormalPrismBlindBoreSpec {
    /// World-space center on the selected entry cap.
    pub center: Point3,
    pub radius: f64,
    pub depth: f64,
    /// Entry side relative to the signed supplied extrusion axis.
    pub entry: NormalPrismBoreEntry,
}

#[derive(Clone, Debug)]
pub struct NormalPrismBlindBores {
    kept: Solid,
    removed: Vec<Solid>,
    hole_faces: Vec<[usize; 4]>,
    floor_faces: Vec<usize>,
    direct_removed_volume: f64,
}
impl NormalPrismBlindBores {
    pub fn kept(&self) -> &Solid {
        &self.kept
    }
    /// Independent removed tools, in input order.
    pub fn removed(&self) -> &[Solid] {
        &self.removed
    }
    pub fn hole_faces(&self) -> &[[usize; 4]] {
        &self.hole_faces
    }
    pub fn floor_faces(&self) -> &[usize] {
        &self.floor_faces
    }
    /// Compensated sum of the positive analytic cylinder volumes.
    pub fn direct_removed_volume(&self) -> f64 {
        self.direct_removed_volume
    }
    pub fn into_solids(self) -> (Solid, Vec<Solid>) {
        (self.kept, self.removed)
    }
}
const UNRESOLVED: Error =
    Error::Unsupported("multiple blind bore graft or arithmetic is unresolved");
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
fn same_frame(a: Frame3, b: Frame3) -> bool {
    a.origin() == b.origin() && a.axes() == b.axes()
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    match (a, b) {
        (Curve::Line { a: a0, b: a1 }, Curve::Line { a: b0, b: b1 }) => a0 == b0 && a1 == b1,
        (
            Curve::Arc {
                frame: a,
                radius: ar,
                sweep: asw,
            },
            Curve::Arc {
                frame: b,
                radius: br,
                sweep: bsw,
            },
        ) => same_frame(*a, *b) && ar == br && asw == bsw,
        _ => false,
    }
}
fn same_surface(a: &Surface, b: &Surface) -> bool {
    match (a, b) {
        (
            Surface::Plane {
                origin: a,
                u: au,
                v: av,
            },
            Surface::Plane {
                origin: b,
                u: bu,
                v: bv,
            },
        ) => a == b && au == bu && av == bv,
        (
            Surface::Cylinder {
                center: a,
                radius: ar,
                height: ah,
            },
            Surface::Cylinder {
                center: b,
                radius: br,
                height: bh,
            },
        ) => a == b && ar == br && ah == bh,
        (
            Surface::FramedCylinder {
                frame: a,
                radius: ar,
                height: ah,
            },
            Surface::FramedCylinder {
                frame: b,
                radius: br,
                height: bh,
            },
        ) => same_frame(*a, *b) && ar == br && ah == bh,
        _ => false,
    }
}
fn same_wire(a: &Wire, b: &Wire) -> bool {
    a.coedges.len() == b.coedges.len()
        && a.coedges.iter().zip(&b.coedges).all(|(a, b)| {
            a.edge == b.edge && a.forward == b.forward && same_pcurve(&a.pcurve, &b.pcurve)
        })
}
fn same_pcurve(a: &PCurve, b: &PCurve) -> bool {
    match (a, b) {
        (
            PCurve::Affine {
                origin: a,
                direction: ad,
            },
            PCurve::Affine {
                origin: b,
                direction: bd,
            },
        ) => a == b && ad == bd,
        (
            PCurve::Arc {
                center: a,
                radius: ar,
                start_angle: aa,
                sweep: asw,
            },
            PCurve::Arc {
                center: b,
                radius: br,
                start_angle: ba,
                sweep: bsw,
            },
        ) => a == b && ar == br && aa == ba && asw == bsw,
        _ => false,
    }
}
fn add_compensated(sum: &mut f64, correction: &mut f64, value: f64) -> Result<()> {
    let adjusted = value - *correction;
    let next = *sum + adjusted;
    *correction = (next - *sum) - adjusted;
    *sum = next;
    if sum.is_finite() && correction.is_finite() {
        Ok(())
    } else {
        Err(UNRESOLVED)
    }
}
fn source_prefix(source: &Solid, single: &Solid) -> Result<usize> {
    if single.vertices.len() != source.vertices.len() + 8
        || single.edges.len() != source.edges.len() + 12
        || single.shell.faces.len() != source.shell.faces.len() + 5
    {
        return Err(UNRESOLVED);
    }
    if source
        .vertices
        .iter()
        .zip(&single.vertices)
        .any(|(a, b)| a.point != b.point)
        || source
            .edges
            .iter()
            .zip(&single.edges)
            .any(|(a, b)| a.vertices != b.vertices || !same_curve(&a.curve, &b.curve))
    {
        return Err(UNRESOLVED);
    }
    let mut entry = None;
    for (i, (a, b)) in source
        .shell
        .faces
        .iter()
        .zip(&single.shell.faces)
        .enumerate()
    {
        if a.orientation != b.orientation
            || !same_surface(&a.surface, &b.surface)
            || b.wires.len() < a.wires.len()
            || a.wires.iter().zip(&b.wires).any(|(a, b)| !same_wire(a, b))
        {
            return Err(UNRESOLVED);
        }
        if b.wires.len() != a.wires.len()
            && (b.wires.len() != a.wires.len() + 1 || entry.replace(i).is_some())
        {
            return Err(UNRESOLVED);
        }
    }
    entry.ok_or(UNRESOLVED)
}

/// Create 1–16 blind cavities in certified normal line/quarter-arc or all-line
/// stock. Existing through openings are retained, but existing blind floors are
/// unsupported. Projected disk footprints must remain strictly disjoint even
/// for opposite entry sides; overlapping pockets with a remaining web are not
/// admitted. All source geometry is retained, and removed tools follow input order.
/// The result is outside uniform normal-prism operations. The opt-in bounded
/// analytic STEP reader separately certifies supported batch results.
pub fn blind_bores_normal_prism(
    source: &Solid,
    specs: &[NormalPrismBlindBoreSpec],
    extrusion_axis: Vec3,
    tolerance: GeometryTolerance,
) -> Result<NormalPrismBlindBores> {
    if !(1..=16).contains(&specs.len()) {
        return Err(Error::InvalidInput(
            "multiple blind bores require 1 through 16 pockets",
        ));
    }
    let tol = tolerance.absolute();
    source.validate(tol)?;
    let certificate = crate::arc_line_prism_validation::recognize_validated_normal_prism(
        source,
        extrusion_axis,
        tol,
    )?;
    let segments = certificate.region.outer.len()
        + certificate.region.holes.iter().map(Vec::len).sum::<usize>();
    if segments + 4 * specs.len() > 128 || certificate.region.holes.len() + specs.len() > 16 {
        return Err(Error::Unsupported(
            "multiple blind bore profile or opening resource limit exceeded",
        ));
    }
    let scale = length(source.bounds().max - source.bounds().min);
    let band = tolerance.length_at_scale(scale)?;
    let mut world = scale;
    let mut include = |p: Point3| {
        world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
    };
    for vertex in &source.vertices {
        include(vertex.point);
    }
    for edge in &source.edges {
        match edge.curve {
            Curve::Line { a, b } => {
                include(a);
                include(b);
            }
            Curve::Arc { frame, .. } => include(frame.origin()),
            _ => return Err(UNRESOLVED),
        }
    }
    for face in &source.shell.faces {
        match face.surface {
            Surface::Plane { origin, .. } => include(origin),
            Surface::Cylinder { center, .. } => include(center),
            Surface::FramedCylinder { frame, .. } => include(frame.origin()),
            _ => return Err(UNRESOLVED),
        }
    }
    let mut centers = Vec::with_capacity(specs.len());
    for spec in specs {
        if !spec.center.finite()
            || !spec.radius.is_finite()
            || spec.radius <= 0.
            || !spec.depth.is_finite()
            || spec.depth <= 0.
        {
            return Err(Error::InvalidInput(
                "blind bore centers, positive radii and depths must be finite",
            ));
        }
        include(spec.center);
        let local = certificate.frame.local_point(spec.center);
        if !local.finite() {
            return Err(UNRESOLVED);
        }
        centers.push([local.x, local.y]);
        // Every actual tool remains within stock, but reserve the requested
        // center/depth arithmetic before delegating cap projection to the single API.
        include(spec.center + certificate.frame.axes()[2] * spec.depth);
        include(spec.center - certificate.frame.axes()[2] * spec.depth);
    }
    for spec in specs {
        world = world.max(spec.radius).max(spec.depth);
    }
    let arithmetic = 4096. * f64::EPSILON * world;
    if !arithmetic.is_finite() || arithmetic >= tolerance.linear() / 256. {
        return Err(UNRESOLVED);
    }
    let clearance = 10. * band + 2. * arithmetic;
    if !clearance.is_finite() {
        return Err(UNRESOLVED);
    }
    for i in 0..specs.len() {
        for j in 0..i {
            let distance = (centers[i][0] - centers[j][0]).hypot(centers[i][1] - centers[j][1]);
            let gap = distance - specs[i].radius - specs[j].radius;
            if !gap.is_finite() || gap <= clearance {
                return Err(Error::InvalidInput(
                    "blind bore projected disks must be strictly disjoint",
                ));
            }
        }
    }
    let mut kept = source.clone();
    let mut removed = Vec::with_capacity(specs.len());
    let mut hole_faces = Vec::with_capacity(specs.len());
    let mut floor_faces = Vec::with_capacity(specs.len());
    let mut direct_total = 0.;
    let mut direct_correction = 0.;
    let mut measured_total = 0.;
    let mut measured_correction = 0.;
    for spec in specs {
        let single = blind_bore_normal_prism(
            source,
            spec.center,
            spec.radius,
            spec.depth,
            extrusion_axis,
            spec.entry,
            tolerance,
        )?;
        let body = single.kept();
        let entry = source_prefix(source, body)?;
        let vertex_offset = kept.vertices.len() - source.vertices.len();
        let edge_offset = kept.edges.len() - source.edges.len();
        let face_offset = kept.shell.faces.len() - source.shell.faces.len();
        kept.vertices
            .extend(body.vertices[source.vertices.len()..].iter().cloned());
        for edge in &body.edges[source.edges.len()..] {
            if edge
                .vertices
                .iter()
                .any(|v| *v < source.vertices.len() || *v >= body.vertices.len())
            {
                return Err(UNRESOLVED);
            }
            let mut edge = edge.clone();
            edge.vertices = edge.vertices.map(|v| v + vertex_offset);
            kept.edges.push(edge);
        }
        let mut entry_wire = body.shell.faces[entry]
            .wires
            .last()
            .ok_or(UNRESOLVED)?
            .clone();
        for c in &mut entry_wire.coedges {
            if c.edge < source.edges.len() || c.edge >= body.edges.len() {
                return Err(UNRESOLVED);
            }
            c.edge += edge_offset;
        }
        kept.shell.faces[entry].wires.push(entry_wire);
        for face in &body.shell.faces[source.shell.faces.len()..] {
            let mut face = face.clone();
            for wire in &mut face.wires {
                for c in &mut wire.coedges {
                    if c.edge < source.edges.len() || c.edge >= body.edges.len() {
                        return Err(UNRESOLVED);
                    }
                    c.edge += edge_offset;
                }
            }
            kept.shell.faces.push(face);
        }
        let ids = *single.hole_faces();
        let floor = single.floor_face();
        if ids
            .iter()
            .chain(std::iter::once(&floor))
            .any(|id| *id < source.shell.faces.len() || *id >= body.shell.faces.len())
        {
            return Err(UNRESOLVED);
        }
        hole_faces.push(ids.map(|id| id + face_offset));
        floor_faces.push(floor + face_offset);
        single.removed().validate(tol)?;
        let actual = single.removed().volume()?;
        let direct = single.direct_removed_volume();
        if !actual.is_finite()
            || actual <= 0.
            || !direct.is_finite()
            || direct < f64::MIN_POSITIVE
            || (actual - direct).abs() > 8192. * f64::EPSILON * actual.max(direct)
        {
            return Err(UNRESOLVED);
        }
        add_compensated(&mut direct_total, &mut direct_correction, direct)?;
        add_compensated(&mut measured_total, &mut measured_correction, actual)?;
        removed.push(single.removed().clone());
    }
    kept.validate(tol)?;
    let before = source.volume()?;
    let after = kept.volume()?;
    let volume_budget = 8192. * f64::EPSILON * before.abs().max(after.abs()).max(measured_total);
    if !before.is_finite()
        || before <= 0.
        || !after.is_finite()
        || after <= 0.
        || !volume_budget.is_finite()
        || (before - after - measured_total).abs() > volume_budget
        || (direct_total - measured_total).abs()
            > 8192. * f64::EPSILON * direct_total.max(measured_total)
    {
        return Err(UNRESOLVED);
    }
    Ok(NormalPrismBlindBores {
        kept,
        removed,
        hole_faces,
        floor_faces,
        direct_removed_volume: direct_total,
    })
}
