//! Exact source-vertical plane partitions shared by native and WASM demos.
use crate::*;
/// Payload: 13 model values, directed UV line start/end, selected side (0/1),
/// followed optionally by 3..16 convex CCW polygon UV pairs.
pub fn nurbs_graph_polygon_split_demo_json(values: &[f64]) -> Result<String> {
    if !(values.len() == 18
        || ((24..=50).contains(&values.len()) && values.len().is_multiple_of(2)))
        || values.iter().any(|v| !v.is_finite())
        || !matches!(values[17], 0. | 1.)
    {
        return Err(Error::InvalidInput("line split payload requires 18 finite model/line/side values and optional 3..16 UV pairs"));
    }
    let x = values;
    let tol = Tolerance::default();
    let source = crate::nurbs_graph_classification_demo::query_source(
        [x[0], x[1], x[2]],
        x[3],
        x[4],
        x[5],
        [x[6], x[7], x[8]],
        [[x[9], x[10]], [x[11], x[12]]],
    )?;
    let start = [x[13], x[14]];
    let end = [x[15], x[16]];
    let polygon = if x.len() == 18 {
        vec![[x[9], x[11]], [x[10], x[11]], [x[10], x[12]], [x[9], x[12]]]
    } else {
        x[18..].as_chunks::<2>().0.to_vec()
    };
    let body = NurbsGraphPolygonSolid::new(&source, polygon.clone(), tol)?;
    let split = if x.len() == 18 {
        source.split_uv_line(start, end, tol)?
    } else {
        body.split_uv_line(start, end, tol)?
    };
    split.validate(tol)?;
    let child_metadata = |child: &NurbsGraphPolygonSolid| -> Result<serde_json::Value> {
        let mass = child.mass_properties(tol)?;
        Ok(
            serde_json::json!({"polygon":child.polygon(),"volume":mass.volume,"centroid":[mass.centroid.x,mass.centroid.y,mass.centroid.z],"brep":crate::nurbs_graph_polygon_demo::serialize_polygon_brep(child)?}),
        )
    };
    let negative = child_metadata(&split.negative)?;
    let positive = child_metadata(&split.positive)?;
    let xyz = |p: Point3| [p.x, p.y, p.z];
    let vec = |v: Vec3| [v.x, v.y, v.z];
    let curves = split.section.edges.iter().map(|e| match &e.curve {
        Curve::Nurbs(c) => Ok(serde_json::json!({"degree":c.degree(),"knots":c.knots(),"weights":c.weights(),"control_points":c.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()})),
        _=>Err(Error::Unsupported("line split section curve unsupported"))
    }).collect::<Result<Vec<_>>>()?;
    let samples = split
        .section
        .edges
        .iter()
        .map(|e| match &e.curve {
            Curve::Nurbs(c) => Ok(c
                .tessellate_bounded(x[4], 65536)?
                .points
                .iter()
                .flat_map(|p| xyz(*p))
                .collect::<Vec<_>>()),
            _ => Err(Error::Unsupported("line split section curve unsupported")),
        })
        .collect::<Result<Vec<_>>>()?;
    let Surface::Nurbs(surface) = &split.section.face.surface else {
        return Err(Error::InvalidTopology("line split section is not NURBS"));
    };
    let pcurves = split.section.face.wires[0]
        .coedges
        .iter()
        .map(|c| match c.pcurve {
            PCurve::Affine { origin, direction } => {
                Ok(serde_json::json!({"origin":origin,"direction":direction}))
            }
            _ => Err(Error::Unsupported("line split section pcurve unsupported")),
        })
        .collect::<Result<Vec<_>>>()?;
    let Surface::Plane { origin, u, v } = &split.plane else {
        return Err(Error::InvalidTopology("line split plane is not planar"));
    };
    let child = if x[17] == 0. {
        &split.negative
    } else {
        &split.positive
    };
    let text = crate::nurbs_graph_polygon_demo::serialize_polygon_graph(
        child, x[0], x[1], x[2], x[3], x[4],
    )?;
    let mut selected: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| Error::InvalidInput("split display serialization failed"))?;
    selected["placement"] =
        serde_json::json!({"angle":x[5],"translation":[x[6],x[7],x[8]],"axis":[0.,1.,0.]});
    selected["scope"]=serde_json::json!("exact source-vertical infinite-plane partition of a convex graph solid; generic Booleans remain unsupported");
    selected["split"] = serde_json::json!({"start":start,"end":end,"side":x[17] as usize,"source_polygon":polygon,"source_volume":body.volume()?,"negative_volume":split.negative.volume()?,"positive_volume":split.positive.volume()?,"negative":negative,"positive":positive,"plane":{"origin":xyz(*origin),"u":vec(*u),"v":vec(*v),"normal":vec(u.cross(*v).normalized()?)},"section":{"closed":false,"vertices":split.section.vertices.iter().map(|v|xyz(v.point)).collect::<Vec<_>>(),"edge_vertices":split.section.edges.iter().map(|e|e.vertices).collect::<Vec<_>>(),"curves":curves,"boundary_samples":samples,"pcurves":pcurves,"orientation":split.section.face.orientation,"edge_forwards":split.section.face.wires[0].coedges.iter().map(|c|c.forward).collect::<Vec<_>>(),"surface":{"kind":"nurbs","degrees":surface.degrees(),"knots":[surface.knots(0)?,surface.knots(1)?],"counts":surface.control_counts(),"domain":surface.domain(),"weights":surface.weights(),"control_points":surface.control_points().iter().map(|p|xyz(*p)).collect::<Vec<_>>()}}});
    Ok(selected.to_string())
}
