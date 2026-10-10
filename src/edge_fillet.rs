//! Circular fillets on selected parallel edges of recognized orthogonal box B-reps.
use crate::*;
use std::f64::consts::{FRAC_PI_2, PI};
fn rotate(p: [f64; 2], k: usize) -> [f64; 2] {
    match k % 4 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}
#[derive(Clone, Debug)]
pub struct ParallelBoxEdgeFillets {
    solid: Solid,
    selections: Vec<(usize, f64)>,
    fillet_faces: Vec<usize>,
    removed_volume: f64,
}
impl ParallelBoxEdgeFillets {
    pub fn solid(&self) -> &Solid {
        &self.solid
    }
    /// Original input edge indices and physical radii, in request order.
    pub fn selections(&self) -> &[(usize, f64)] {
        &self.selections
    }
    /// Actual cylindrical face indices in the returned solid.
    pub fn fillet_faces(&self) -> &[usize] {
        &self.fillet_faces
    }
    /// Analytic removed volume; no cusp-bearing removed solid is implied.
    pub fn removed_volume(&self) -> f64 {
        self.removed_volume
    }
    pub fn into_solid(self) -> Solid {
        self.solid
    }
}
fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}
/// Replace 1..4 parallel original box edges by tangent quarter cylinders.
/// Only validated eight-corner orthogonal box B-reps are recognized. Radii must
/// leave resolved straight segments; mixed axes and already curved bodies are unsupported.
/// Frame recovery uses reserved physical precision, not arbitrary shape repair.
pub fn fillet_parallel_box_edges(
    source: &Solid,
    selections: &[(usize, f64)],
    tol: GeometryTolerance,
) -> Result<ParallelBoxEdgeFillets> {
    source.validate(tol.absolute())?;
    if source.vertices.len() != 8
        || source.edges.len() != 12
        || source.shell.faces.len() != 6
        || source.shell.faces.iter().any(|f| {
            !matches!(f.surface, Surface::Plane { .. })
                || f.wires.len() != 1
                || f.wires[0].coedges.len() != 4
        })
        || source
            .edges
            .iter()
            .any(|e| !matches!(e.curve, Curve::Line { .. }))
    {
        return Err(Error::Unsupported("fillet supports orthogonal box topology with 8 vertices, 12 lines and 6 planar quadrilateral faces"));
    }
    if selections.is_empty() || selections.len() > 4 {
        return Err(Error::InvalidInput(
            "fillet requires 1..4 original parallel box edges",
        ));
    }
    let first = source
        .edges
        .get(selections[0].0)
        .ok_or(Error::InvalidInput(
            "fillet original edge index is out of range",
        ))?;
    let selected = (source.vertices[first.vertices[1]].point
        - source.vertices[first.vertices[0]].point)
        .normalized()?;
    let origin = source.vertices[0].point;
    let incident: Vec<Vec3> = source
        .edges
        .iter()
        .filter_map(|e| {
            if e.vertices[0] == 0 {
                Some(source.vertices[e.vertices[1]].point - origin)
            } else if e.vertices[1] == 0 {
                Some(source.vertices[e.vertices[0]].point - origin)
            } else {
                None
            }
        })
        .collect();
    if incident.len() != 3 {
        return Err(Error::Unsupported(
            "fillet source corner must have three incident edges",
        ));
    }
    let incident_axes = incident
        .iter()
        .map(|a| a.normalized())
        .collect::<Result<Vec<_>>>()?;
    let zindex = incident_axes
        .iter()
        .position(|a| length(a.cross(selected)) <= tol.angular().sin())
        .ok_or(Error::Unsupported(
            "fillet selected edge does not match a box axis",
        ))?;
    let z = incident[zindex].normalized()?;
    let mut xy: Vec<Vec3> = incident
        .iter()
        .enumerate()
        .filter_map(|(i, &a)| if i != zindex { Some(a) } else { None })
        .collect();
    if xy[0].cross(xy[1]).dot(z) < 0. {
        xy.swap(0, 1);
    }
    let width = length(xy[0]);
    let depth = length(xy[1]);
    let height = length(incident[zindex]);
    let raw_axes = [xy[0].normalized()?, xy[1].normalized()?, z];
    let frame = Frame3::new_with_tolerance(origin, raw_axes, tol)?;
    let extent = width.hypot(depth).hypot(height);
    let band = tol.length_at_scale(extent)?;
    let world = source.vertices.iter().fold(extent, |m, v| {
        m.max(v.point.x.abs())
            .max(v.point.y.abs())
            .max(v.point.z.abs())
    });
    let world = source.shell.faces.iter().try_fold(world, |m, f| {
        let Surface::Plane { origin, u, v } = f.surface else {
            return Err(Error::Unsupported("fillet source requires planar faces"));
        };
        Frame3::new_with_tolerance(origin, [u, v, u.cross(v)], tol)?;
        Ok::<_, Error>(
            m.max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs()),
        )
    })?;
    let arith = 4096. * f64::EPSILON * world;
    let angular_drift = raw_axes
        .iter()
        .zip(frame.axes())
        .map(|(&a, b)| length(a - b))
        .fold(0., f64::max)
        * extent;
    if !angular_drift.is_finite() || angular_drift + arith >= tol.linear() / 8. {
        return Err(Error::Unsupported(
            "fillet source frame reconstruction is unresolved",
        ));
    }
    if !arith.is_finite() || arith >= tol.linear() / 8. {
        return Err(Error::Unsupported(
            "fillet world-coordinate precision is unresolved",
        ));
    }
    let mut codes: Vec<u8> = Vec::new();
    for vertex in &source.vertices {
        let p = frame.local_point(vertex.point);
        let xyz = [p.x, p.y, p.z];
        let sizes = [width, depth, height];
        let mut code = 0u8;
        let mut local = [0.; 3];
        for i in 0..3 {
            if (xyz[i] - sizes[i]).abs() < xyz[i].abs() {
                code |= 1 << i;
                local[i] = sizes[i];
            }
        }
        let expected = frame.point(Point3::new(local[0], local[1], local[2]));
        if length(vertex.point - expected) + arith + angular_drift >= tol.linear() / 4. {
            return Err(Error::Unsupported(
                "fillet source corners do not form a resolved orthogonal box",
            ));
        }
        codes.push(code);
    }
    let unique: std::collections::BTreeSet<_> = codes.iter().copied().collect();
    if unique.len() != 8 {
        return Err(Error::Unsupported(
            "fillet source must contain eight distinct cube corners",
        ));
    }
    let mut edge_keys = std::collections::BTreeSet::new();
    for e in &source.edges {
        let a = codes[e.vertices[0]];
        let b = codes[e.vertices[1]];
        if (a ^ b).count_ones() != 1 || !edge_keys.insert((a.min(b), a.max(b))) {
            return Err(Error::Unsupported(
                "fillet source does not have unique cube edges",
            ));
        }
    }
    let mut face_keys = std::collections::BTreeSet::new();
    for f in &source.shell.faces {
        let own: std::collections::BTreeSet<_> = f.wires[0]
            .coedges
            .iter()
            .flat_map(|c| source.edges[c.edge].vertices)
            .collect();
        if own.len() != 4 {
            return Err(Error::Unsupported(
                "fillet source must have four distinct corners per face",
            ));
        }
        let mut and = 7u8;
        let mut or = 0u8;
        for &i in &own {
            and &= codes[i];
            or |= codes[i];
        }
        let constant = and | (!or & 7);
        if constant.count_ones() != 1 || !face_keys.insert((constant, and)) {
            return Err(Error::Unsupported(
                "fillet source does not have unique cube faces",
            ));
        }
        let axis = constant.trailing_zeros() as usize;
        let expected = frame.axes()[axis] * if and != 0 { 1. } else { -1. };
        let Surface::Plane { origin, u, v } = f.surface else {
            unreachable!()
        };
        let outward = u.cross(v).normalized()? * f64::from(f.orientation);
        if length(outward - expected) * extent + arith >= tol.linear() / 4. {
            return Err(Error::Unsupported(
                "fillet supporting face orientation is unresolved",
            ));
        }
        for &i in &own {
            let residual = (source.vertices[i].point - origin).dot(outward);
            if !residual.is_finite() || residual.abs() + arith >= tol.linear() / 4. {
                return Err(Error::Unsupported(
                    "fillet source face loses plane precision",
                ));
            }
        }
    }
    let corners = [[0., 0.], [width, 0.], [width, depth], [0., depth]];
    let mut radii = [0.; 4];
    for &(edge, r) in selections {
        let e = source.edges.get(edge).ok_or(Error::InvalidInput(
            "fillet original edge index is out of range",
        ))?;
        let [a, b] = e.vertices;
        if codes[a] ^ codes[b] != 4 {
            return Err(Error::Unsupported(
                "fillet selected edges must span one common box axis",
            ));
        }
        let xycode = codes[a] & 3;
        let k = match xycode {
            0 => 0,
            1 => 1,
            3 => 2,
            _ => 3,
        };
        if radii[k] != 0. || !r.is_finite() || r <= 10. * band {
            return Err(Error::InvalidInput(
                "fillet radius must be positive, resolved and selected only once",
            ));
        }
        radii[k] = r;
    }
    for k in 0..4 {
        let side = if k % 2 == 0 { width } else { depth };
        if radii[k] + radii[(k + 1) % 4] >= side - 10. * band {
            return Err(Error::Unsupported(
                "fillet neighboring radii leave no resolved straight boundary",
            ));
        }
    }
    let point = |k: usize, p: [f64; 2]| {
        let q = rotate(p, k);
        [corners[k][0] + q[0], corners[k][1] + q[1]]
    };
    let mut segments = Vec::new();
    for k in 0..4 {
        let r = radii[k];
        if r > 0. {
            segments.push(PlanarSegment::Arc {
                center: point(k, [r, r]),
                radius: r,
                start_angle: (PI + k as f64 * FRAC_PI_2).rem_euclid(2. * PI),
                sweep: FRAC_PI_2,
            });
        }
        segments.push(PlanarSegment::Line {
            a: point(k, [r, 0.]),
            b: point((k + 1) % 4, [0., radii[(k + 1) % 4]]),
        });
    }
    let kept = extrude_arc_line_in_frame(
        &ArcLineProfile {
            origin: Point3::new(0., 0., 0.),
            segments,
        },
        frame.axes()[2] * height,
        frame,
        tol.absolute(),
    )?;
    let fillet_faces: Vec<_> = kept
        .shell
        .faces
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            if matches!(
                f.surface,
                Surface::FramedCylinder { .. } | Surface::Cylinder { .. }
            ) {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    if fillet_faces.len() != selections.len() {
        return Err(Error::InvalidTopology(
            "fillet result has an unexpected cylindrical face count",
        ));
    }
    let mut removed_volume = 0.;
    for &r in &radii {
        if r > 0. {
            let v = (1. - PI / 4.) * r * r * height;
            if !v.is_normal() || v <= 0. {
                return Err(Error::Unsupported(
                    "fillet removed volume exceeds resolved numeric range",
                ));
            }
            removed_volume += v;
        }
    }
    let original = source.volume()?;
    let total = kept.volume()? + removed_volume;
    if !total.is_finite() || (total - original).abs() > original.abs() * 1e-10 {
        return Err(Error::Unsupported(
            "fillet source recognition does not preserve volume within numeric budget",
        ));
    }
    kept.validate(tol.absolute())?;
    Ok(ParallelBoxEdgeFillets {
        solid: kept,
        selections: selections.to_vec(),
        fillet_faces,
        removed_volume,
    })
}
