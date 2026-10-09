//! Strict recognition of an unplaced full-domain convex graph polygon from STEP.
use crate::nurbs_graph_step_import::{reindex, same_curve, same_surface};
use crate::*;
use std::collections::BTreeSet;
fn unsupported() -> Error {
    Error::Unsupported("STEP is not an exact unplaced full-domain canonical convex graph polygon")
}
fn neighbors(seed: f64) -> Vec<f64> {
    let mut v = vec![seed];
    let (mut lo, mut hi) = (seed, seed);
    for _ in 0..64 {
        lo = lo.next_down();
        hi = hi.next_up();
        if lo.is_finite() {
            v.push(lo);
        }
        if hi.is_finite() {
            v.push(hi);
        }
    }
    v
}
/// Import the actual retained spline geometry of one canonical convex graph polygon.
/// Rational weights are preserved; affine uses must be exact preimages of the writer.
pub fn import_step_nurbs_graph_polygon_mm(
    input: &str,
    tol: Tolerance,
) -> Result<NurbsGraphPolygonSolid> {
    Tolerance::new(tol.linear)?;
    let parsed = crate::nurbs_graph_polygon_step_read::read_polygon_solid(input, tol)?;
    let actual = &parsed.solid;
    for (face_index, face) in actual.shell.faces.iter().enumerate() {
        let Surface::Nurbs(roof) = &face.surface else {
            continue;
        };
        if face.orientation != 1
            || face.wires.len() != 1
            || roof.degrees() != [2, 2]
            || roof.control_counts() != [3, 3]
            || roof.domain() != [[0., 1.], [0., 1.]]
            || roof.weights() != [1.; 9]
        {
            continue;
        }
        let p = roof.control_points();
        let dimensions = [p[6].x, p[2].y, p[0].z];
        if dimensions.iter().any(|x| !x.is_finite() || *x <= 0.) {
            continue;
        }
        let wire = &face.wires[0];
        if !(3..=16).contains(&wire.coedges.len()) {
            continue;
        }
        // Canonical polygon cap edges run from each corner to its successor.
        // Reversed geometry/parameter bases are outside this recognition scope.
        let mut polygon = Vec::new();
        let mut valid = true;
        for c in &wire.coedges {
            if !c.forward {
                valid = false;
                break;
            }
            let Some(raw) = parsed.affine.get(&(face_index, c.edge)) else {
                valid = false;
                break;
            };
            polygon.push(raw.origin);
        }
        if !valid {
            continue;
        }
        let first = (0..polygon.len())
            .min_by(|&a, &b| {
                polygon[a][0]
                    .total_cmp(&polygon[b][0])
                    .then(polygon[a][1].total_cmp(&polygon[b][1]))
            })
            .unwrap();
        polygon.rotate_left(first);
        let mut tried = BTreeSet::new();
        // Recover the encoded original center coefficient, not an input recipe.
        for center in neighbors(p[4].z) {
            let b = center - dimensions[2];
            if !b.is_finite() || !tried.insert(b.to_bits()) {
                continue;
            }
            let Ok(source) = NurbsGraphSolid::new(dimensions, b, tol) else {
                continue;
            };
            if !same_surface(&face.surface, &source.brep().shell.faces[1].surface)? {
                continue;
            }
            let Ok(mut candidate) = NurbsGraphPolygonSolid::new(&source, polygon.clone(), tol)
            else {
                continue;
            };
            let mut restored = actual.clone();
            let mut matches = true;
            for (fi, af) in restored.shell.faces.iter_mut().enumerate() {
                let mut ef = None;
                for f in &candidate.brep().shell.faces {
                    if same_surface(&af.surface, &f.surface)? {
                        ef = Some(f);
                        break;
                    }
                }
                let Some(ef) = ef else {
                    matches = false;
                    break;
                };
                for wire in &mut af.wires {
                    for coedge in &mut wire.coedges {
                        let Some(edge) = actual.edges.get(coedge.edge) else {
                            matches = false;
                            break;
                        };
                        let Some(ei) = candidate
                            .brep()
                            .edges
                            .iter()
                            .position(|e| same_curve(&edge.curve, &e.curve))
                        else {
                            matches = false;
                            break;
                        };
                        let Some(ec) = ef
                            .wires
                            .iter()
                            .flat_map(|w| &w.coedges)
                            .find(|c| c.edge == ei)
                        else {
                            matches = false;
                            break;
                        };
                        let PCurve::Affine { origin, direction } = &ec.pcurve else {
                            matches = false;
                            break;
                        };
                        let Some(raw) = parsed.affine.get(&(fi, coedge.edge)) else {
                            matches = false;
                            break;
                        };
                        let (ratios, magnitude) =
                            crate::nurbs_graph_step::decompose_affine(*origin, *direction)?;
                        if raw.origin != *origin
                            || raw.direction != ratios
                            || raw.magnitude != magnitude
                        {
                            matches = false;
                            break;
                        }
                        coedge.pcurve = ec.pcurve.clone();
                    }
                    if !matches {
                        break;
                    }
                }
                if !matches {
                    break;
                }
            }
            if !matches {
                continue;
            }
            let Ok(restored) = reindex(restored, candidate.brep()) else {
                continue;
            };
            candidate.solid = restored;
            if candidate.validate(tol).is_ok() {
                return Ok(candidate);
            }
        }
    }
    Err(unsupported())
}
