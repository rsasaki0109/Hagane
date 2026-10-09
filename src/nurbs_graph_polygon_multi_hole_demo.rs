//! Checked numeric transport for a convex graph stock with 1..4 polygon openings.
use crate::*;

/// Model13, outer count (0 rectangle or 3..16), hole count (1..4), outer
/// UV pairs, then each hole's count (3..16) and UV pairs. Total corners,
/// including an implicit rectangle's four, are limited to 64; at most 147 values.
pub fn nurbs_graph_polygon_multi_hole_demo_json(x: &[f64]) -> Result<String> {
    let body = polygon_multi_hole_model(x)?;
    let tol = Tolerance::default();
    let source = body.source();
    let outer = NurbsGraphPolygonSolid::new(source, body.outer_polygon().to_vec(), tol)?;
    let display = body.tessellate_bounded(x[4], 65536, tol)?;
    let mass = body.mass_properties(tol)?;
    let (inertia_properties, inertia_error) =
        crate::nurbs_graph_polygon_demo::serialize_polygon_inertia(body.inertia_properties(tol));
    let bounds = body.bounds()?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let positions: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|id| xyz(display.mesh.positions[*id])))
        .collect();
    let normals: Vec<_> = display
        .mesh
        .triangles
        .iter()
        .flat_map(|t| {
            t.iter().flat_map(|id| {
                let n = display.mesh.normals[*id];
                [n.x, n.y, n.z]
            })
        })
        .collect();
    let boundary_samples = body
        .brep()
        .edges
        .iter()
        .map(|edge| match &edge.curve {
            Curve::Line { a, b } => Ok(vec![a.x, a.y, a.z, b.x, b.y, b.z]),
            Curve::Nurbs(curve) => Ok(curve
                .tessellate_bounded(x[4], 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect::<Vec<_>>()),
            _ => Err(Error::Unsupported(
                "multiple polygon openings demo boundary curve unsupported",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let removed_volumes = body
        .openings()
        .iter()
        .map(|opening| NurbsGraphPolygonSolid::new(source, opening.clone(), tol)?.volume())
        .collect::<Result<Vec<_>>>()?;
    let removed_volume = removed_volumes.iter().sum::<f64>();
    Ok(serde_json::json!({
        "outer_polygon":body.outer_polygon(),"openings":body.openings(),"genus":body.openings().len(),
        "scope":"1..4 disjoint strictly convex CCW polygon through openings in convex graph stock, at most 64 total corners; polygon STEP import and generic Booleans remain unsupported",
        "width":x[0],"depth":x[1],"height":x[2],"bulge":x[3],"error":x[4],"source_domain":source.source_domain(),
        "placement":{"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]},
        "volume":mass.volume,"source_volume":outer.volume()?,"removed_volume":removed_volume,"removed_volumes":removed_volumes,
        "mass_properties":{"volume":mass.volume,"centroid":xyz(mass.centroid),"units":"mm","volume_units":"mm3","centroid_units":"mm","density":"uniform","method":"positive material-triangle polynomial integration"},
        "inertia_properties":inertia_properties,"inertia_error":inertia_error,
        "bounds_kind":"conservative control hull bounds","bounds":{"min":xyz(bounds.min),"max":xyz(bounds.max)},
        "positions":positions,"normals":normals,"boundary_samples":boundary_samples,
        "mesh":{"positions":display.mesh.positions.iter().map(|p|xyz(*p)).collect::<Vec<_>>(),"normals":display.mesh.normals.iter().map(|n|[n.x,n.y,n.z]).collect::<Vec<_>>(),"triangles":display.mesh.triangles,"face_ids":display.mesh.face_ids},
        "vertex_nodes":display.vertex_nodes,"vertex_uv":display.vertex_uv,"vertex_faces":display.vertex_faces,"error_bounds":display.error_bounds,"subdivisions":display.subdivisions,
        "brep":crate::nurbs_graph_polygon_demo::serialize_graph_brep(body.brep())?
    }).to_string())
}

/// Construct and validate retained geometry without requesting display resources.
pub(crate) fn polygon_multi_hole_model(x: &[f64]) -> Result<NurbsGraphPolygonMultiHoledSolid> {
    let (polygon, openings) = decode_polygons(x)?;
    let tol = Tolerance::default();
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let outer = NurbsGraphPolygonSolid::new(&source, polygon, tol)?;
    let body = NurbsGraphPolygonMultiHoledSolid::new(&outer, openings, tol)?;
    body.validate(tol)?;
    Ok(body)
}

type DecodedPolygons = (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>);
fn decode_polygons(x: &[f64]) -> Result<DecodedPolygons> {
    if !(22..=147).contains(&x.len()) || x.iter().any(|v| !v.is_finite()) {
        return Err(Error::InvalidInput(
            "multiple polygon opening payload requires 22..147 finite values",
        ));
    }
    let outer_count = count(x[13], 0, 16)?;
    if outer_count == 1 || outer_count == 2 {
        return Err(Error::InvalidInput(
            "outer polygon count must be zero or an integer in 3..16",
        ));
    }
    let hole_count = count(x[14], 1, 4)?;
    let mut cursor = 15;
    let polygon = if outer_count == 0 {
        vec![[x[9], x[11]], [x[10], x[11]], [x[10], x[12]], [x[9], x[12]]]
    } else {
        pairs(x, &mut cursor, outer_count)?
    };
    let mut total = polygon.len();
    let mut openings = Vec::with_capacity(hole_count);
    for _ in 0..hole_count {
        let value = *x
            .get(cursor)
            .ok_or(Error::InvalidInput("missing polygon opening count"))?;
        cursor += 1;
        let n = count(value, 3, 16)?;
        total += n;
        if total > 64 {
            return Err(Error::InvalidInput(
                "multiple polygon openings exceed 64 total corners",
            ));
        }
        openings.push(pairs(x, &mut cursor, n)?);
    }
    if cursor != x.len() {
        return Err(Error::InvalidInput(
            "multiple polygon opening payload contains trailing values",
        ));
    }
    Ok((polygon, openings))
}
fn count(value: f64, min: usize, max: usize) -> Result<usize> {
    if !value.is_finite() || value.fract() != 0. || value < min as f64 || value > max as f64 {
        return Err(Error::InvalidInput(
            "polygon counts must be finite integers within their supported ranges",
        ));
    }
    Ok(value as usize)
}
fn pairs(x: &[f64], cursor: &mut usize, n: usize) -> Result<Vec<[f64; 2]>> {
    // Counts are checked before conversion and bounded by 16, so these index
    // operations cannot overflow; get() rejects any incomplete transport.
    let end = *cursor + 2 * n;
    let values = x
        .get(*cursor..end)
        .ok_or(Error::InvalidInput("polygon opening payload is truncated"))?;
    *cursor = end;
    Ok(values.as_chunks::<2>().0.to_vec())
}

#[cfg(test)]
mod transport_tests {
    use super::*;
    fn payload(outer: usize, holes: &[usize]) -> Vec<f64> {
        let mut values = vec![
            80.,
            60.,
            20.,
            0.,
            0.5,
            0.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            1.,
            outer as f64,
            holes.len() as f64,
        ];
        values.extend(std::iter::repeat_n(0.5, 2 * outer));
        for &count in holes {
            values.push(count as f64);
            values.extend(std::iter::repeat_n(0.5, 2 * count));
        }
        values
    }
    #[test]
    fn complete_maximum_transport_preserves_all_counts() {
        let values = payload(3, &[15, 15, 15, 16]);
        assert_eq!(values.len(), 147);
        let (outer, holes) = decode_polygons(&values).unwrap();
        assert_eq!(outer.len(), 3);
        assert_eq!(
            holes.iter().map(Vec::len).collect::<Vec<_>>(),
            [15, 15, 15, 16]
        );
        assert_eq!(outer.len() + holes.iter().map(Vec::len).sum::<usize>(), 64);
        assert!(decode_polygons(&payload(0, &[15, 15, 15, 15])).is_ok());
        assert!(decode_polygons(&payload(0, &[16, 16, 16, 16])).is_err());
    }
    #[test]
    fn counts_are_checked_before_integer_conversion_and_geometry() {
        let values = payload(0, &[4, 4]);
        for (index, value) in [
            (13, 1.),
            (13, 2.),
            (13, 3.5),
            (13, f64::MAX),
            (14, 0.),
            (14, 5.),
            (14, 1.5),
            (15, 2.),
            (15, 17.),
            (15, 3.5),
            (15, -1.),
        ] {
            let mut invalid = values.clone();
            invalid[index] = value;
            assert!(
                decode_polygons(&invalid).is_err(),
                "index {index}, value {value}"
            );
        }
        for (index, value) in [
            (0, f64::NAN),
            (13, f64::INFINITY),
            (14, f64::NEG_INFINITY),
            (16, f64::NAN),
        ] {
            let mut invalid = values.clone();
            invalid[index] = value;
            assert!(decode_polygons(&invalid).is_err());
        }
    }
    #[test]
    fn truncated_and_trailing_payloads_never_succeed() {
        let values = payload(0, &[4, 4]);
        for end in 0..values.len() {
            assert!(decode_polygons(&values[..end]).is_err(), "prefix {end}");
        }
        let mut trailing = values;
        trailing.push(0.5);
        assert!(matches!(
            decode_polygons(&trailing),
            Err(Error::InvalidInput(
                "multiple polygon opening payload contains trailing values"
            ))
        ));
    }
}
