//! Bounded raw decoding for exact plain convex polygon graph recognition.
use crate::step_read::{boolean, list, number, reference, Database, Parser, Value};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerializedAffine {
    pub origin: [f64; 2],
    pub direction: [f64; 2],
    pub magnitude: f64,
}
pub(crate) struct ParsedPolygonStep {
    pub solid: Solid,
    pub affine: BTreeMap<(usize, usize), SerializedAffine>,
}
fn unsupported() -> Error {
    Error::Unsupported(
        "STEP polygon graph requires exact supported spline geometry and shared topology",
    )
}
fn empty_components(db: &Database, id: u32, names: &[&str]) -> Result<()> {
    for name in names {
        if !db.component(id, name)?.is_empty() {
            return Err(unsupported());
        }
    }
    let representation = db.component(id, "REPRESENTATION_ITEM")?;
    if representation.len() != 1 || !matches!(representation[0], Value::String(_)) {
        return Err(unsupported());
    }
    Ok(())
}
fn positive_weight(value: &Value) -> Result<f64> {
    let weight = number(value)?;
    if !weight.is_finite() || weight <= 0. {
        return Err(unsupported());
    }
    Ok(weight)
}
fn integer(value: &Value, maximum: usize) -> Result<usize> {
    let v = number(value)?;
    if v < 1. || v > maximum as f64 || v.fract() != 0. {
        return Err(unsupported());
    }
    Ok(v as usize)
}
fn knots(mults: &Value, values: &Value, degree: usize, count: usize) -> Result<Vec<f64>> {
    let m = list(mults)?;
    let k = list(values)?;
    if m.len() != k.len() || m.len() > 64 {
        return Err(unsupported());
    }
    let mut result = Vec::new();
    let mut last = None;
    for (m, k) in m.iter().zip(k) {
        let value = number(k)?;
        let times = integer(m, degree + 1)?;
        if !value.is_finite()
            || last.is_some_and(|last| value <= last)
            || result.len() + times > count + degree + 1
        {
            return Err(unsupported());
        }
        last = Some(value);
        result.extend(std::iter::repeat_n(value, times));
    }
    if result.len() != count + degree + 1 {
        return Err(unsupported());
    }
    Ok(result)
}
fn curve(db: &Database, id: u32, scale: f64) -> Result<NurbsCurve> {
    let (a, raw_weights) = if db.records.get(&id).ok_or_else(unsupported)?.len() == 1 {
        (
            db.simple(id, "B_SPLINE_CURVE_WITH_KNOTS", 9)?.to_vec(),
            None,
        )
    } else {
        empty_components(
            db,
            id,
            &["BOUNDED_CURVE", "CURVE", "GEOMETRIC_REPRESENTATION_ITEM"],
        )?;
        let b = db.component(id, "B_SPLINE_CURVE")?;
        let k = db.component(id, "B_SPLINE_CURVE_WITH_KNOTS")?;
        let w = db.component(id, "RATIONAL_B_SPLINE_CURVE")?;
        if b.len() != 5 || k.len() != 3 || w.len() != 1 {
            return Err(unsupported());
        }
        let mut a = vec![Value::String(String::new())];
        a.extend_from_slice(b);
        a.extend_from_slice(k);
        (a, Some(list(&w[0])?.to_vec()))
    };
    let degree = integer(&a[1], 16)?;
    let points = list(&a[2])?;
    if points.len() > 32
        || a[3] != Value::Enum("UNSPECIFIED".into())
        || boolean(&a[4])?
        || boolean(&a[5])?
        || a[8] != Value::Enum("UNSPECIFIED".into())
    {
        return Err(unsupported());
    }
    let points = points
        .iter()
        .map(|p| db.point(reference(p)?, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = if let Some(values) = raw_weights {
        if values.len() != points.len() {
            return Err(unsupported());
        }
        values
            .iter()
            .map(positive_weight)
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![1.; points.len()]
    };
    let knots = knots(&a[6], &a[7], degree, points.len())?;
    NurbsCurve::new(degree, knots, points, weights)
}
fn surface(db: &Database, id: u32, scale: f64) -> Result<NurbsSurface> {
    let (a, raw_weights) = if db.records.get(&id).ok_or_else(unsupported)?.len() == 1 {
        (
            db.simple(id, "B_SPLINE_SURFACE_WITH_KNOTS", 13)?.to_vec(),
            None,
        )
    } else {
        empty_components(
            db,
            id,
            &[
                "BOUNDED_SURFACE",
                "SURFACE",
                "GEOMETRIC_REPRESENTATION_ITEM",
            ],
        )?;
        let b = db.component(id, "B_SPLINE_SURFACE")?;
        let k = db.component(id, "B_SPLINE_SURFACE_WITH_KNOTS")?;
        let w = db.component(id, "RATIONAL_B_SPLINE_SURFACE")?;
        if b.len() != 7 || k.len() != 5 || w.len() != 1 {
            return Err(unsupported());
        }
        let mut a = vec![Value::String(String::new())];
        a.extend_from_slice(b);
        a.extend_from_slice(k);
        (a, Some(list(&w[0])?.to_vec()))
    };
    let degrees = [integer(&a[1], 16)?, integer(&a[2], 16)?];
    let rows = list(&a[3])?;
    if rows.is_empty()
        || rows.len() > 32
        || a[4] != Value::Enum("UNSPECIFIED".into())
        || (5..8).any(|i| boolean(&a[i]).unwrap_or(true))
        || a[12] != Value::Enum("UNSPECIFIED".into())
    {
        return Err(unsupported());
    }
    let nv = list(&rows[0])?.len();
    if nv == 0 || nv > 32 {
        return Err(unsupported());
    }
    let mut points = Vec::new();
    for row in rows {
        let row = list(row)?;
        if row.len() != nv {
            return Err(unsupported());
        }
        for p in row {
            points.push(db.point(reference(p)?, scale)?);
        }
    }
    let counts = [rows.len(), nv];
    let weights = if let Some(rows) = raw_weights {
        if rows.len() != counts[0] {
            return Err(unsupported());
        }
        let mut weights = Vec::new();
        for row in rows {
            let row = list(&row)?;
            if row.len() != nv {
                return Err(unsupported());
            }
            weights.extend(
                row.iter()
                    .map(positive_weight)
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        weights
    } else {
        vec![1.; points.len()]
    };
    let knot_vectors = [
        knots(&a[8], &a[10], degrees[0], counts[0])?,
        knots(&a[9], &a[11], degrees[1], counts[1])?,
    ];
    NurbsSurface::new(degrees, knot_vectors, counts, points, weights)
}
fn coordinates2(db: &Database, id: u32, kind: &str) -> Result<[f64; 2]> {
    let a = db.simple(id, kind, 2)?;
    let values = list(&a[1])?;
    if values.len() != 2 {
        return Err(unsupported());
    }
    let result = [number(&values[0])?, number(&values[1])?];
    if result.iter().any(|x| !x.is_finite()) {
        return Err(unsupported());
    }
    Ok(result)
}
fn pcurve(db: &Database, id: u32) -> Result<(u32, PCurve, SerializedAffine)> {
    let a = db.simple(id, "PCURVE", 3)?;
    let surface = reference(&a[1])?;
    let representation = db.simple(reference(&a[2])?, "DEFINITIONAL_REPRESENTATION", 3)?;
    let items = list(&representation[1])?;
    if items.len() != 1 {
        return Err(unsupported());
    }
    let context = reference(&representation[2])?;
    if db.component(context, "GEOMETRIC_REPRESENTATION_CONTEXT")? != [Value::Number(2.)]
        || db.component(context, "REPRESENTATION_CONTEXT")?
            != [Value::String(String::new()), Value::String(String::new())]
    {
        return Err(unsupported());
    }
    let line = db.simple(reference(&items[0])?, "LINE", 3)?;
    let origin = coordinates2(db, reference(&line[1])?, "CARTESIAN_POINT")?;
    let vector = db.simple(reference(&line[2])?, "VECTOR", 3)?;
    let direction = coordinates2(db, reference(&vector[1])?, "DIRECTION")?;
    let length = number(&vector[2])?;
    let magnitude = direction[0].hypot(direction[1]);
    if !magnitude.is_finite() || magnitude <= 0. || !length.is_finite() || length <= 0. {
        return Err(unsupported());
    }
    let delta = direction.map(|v| v * length);
    if delta.iter().any(|x| !x.is_finite()) {
        return Err(unsupported());
    }
    Ok((
        surface,
        PCurve::Affine {
            origin,
            direction: delta,
        },
        SerializedAffine {
            origin,
            direction,
            magnitude: length,
        },
    ))
}
type Association = (u32, PCurve, SerializedAffine);
struct Builder<'a> {
    db: &'a Database,
    scale: f64,
    solid: Solid,
    vertices: BTreeMap<u32, usize>,
    edges: BTreeMap<u32, usize>,
    associations: BTreeMap<usize, Vec<Association>>,
    surfaces: Vec<u32>,
    affine: BTreeMap<(usize, usize), SerializedAffine>,
}
impl Builder<'_> {
    fn vertex(&mut self, id: u32) -> Result<usize> {
        if let Some(&v) = self.vertices.get(&id) {
            return Ok(v);
        }
        if self.solid.vertices.len() >= 32 {
            return Err(unsupported());
        }
        let a = self.db.simple(id, "VERTEX_POINT", 2)?;
        let point = self.db.point(reference(&a[1])?, self.scale)?;
        let index = self.solid.vertices.len();
        self.solid.vertices.push(Vertex { point });
        self.vertices.insert(id, index);
        Ok(index)
    }
    fn edge(&mut self, id: u32) -> Result<usize> {
        if let Some(&edge) = self.edges.get(&id) {
            return Ok(edge);
        }
        if self.solid.edges.len() >= 48 {
            return Err(unsupported());
        }
        let a = self.db.simple(id, "EDGE_CURVE", 5)?;
        if !boolean(&a[4])? {
            return Err(unsupported());
        }
        let vertices = [
            self.vertex(reference(&a[1])?)?,
            self.vertex(reference(&a[2])?)?,
        ];
        let geometry = self.db.simple(reference(&a[3])?, "SURFACE_CURVE", 4)?;
        if geometry[3] != Value::Enum("CURVE_3D".into()) {
            return Err(unsupported());
        }
        let curve = curve(self.db, reference(&geometry[1])?, self.scale)?;
        let associated = list(&geometry[2])?;
        if associated.len() != 2 {
            return Err(unsupported());
        }
        let associations = associated
            .iter()
            .map(|a| pcurve(self.db, reference(a)?))
            .collect::<Result<Vec<_>>>()?;
        if associations[0].0 == associations[1].0 {
            return Err(unsupported());
        }
        let index = self.solid.edges.len();
        self.solid.edges.push(Edge {
            vertices,
            curve: Curve::Nurbs(Box::new(curve)),
        });
        self.associations.insert(index, associations);
        self.edges.insert(id, index);
        Ok(index)
    }
    fn face(&mut self, id: u32) -> Result<()> {
        let a = self.db.simple(id, "ADVANCED_FACE", 4)?;
        let bounds = list(&a[1])?;
        if bounds.is_empty() || bounds.len() > 1 {
            return Err(unsupported());
        }
        let surface_id = reference(&a[2])?;
        if self.surfaces.contains(&surface_id) {
            return Err(unsupported());
        }
        let surface = surface(self.db, surface_id, self.scale)?;
        let mut outer = None;
        for bound in bounds {
            let id = reference(bound)?;
            let record = self.db.records.get(&id).ok_or_else(unsupported)?;
            if record.len() != 1 {
                return Err(unsupported());
            }
            let kind = record[0].0.as_str();
            if kind != "FACE_OUTER_BOUND" {
                return Err(unsupported());
            }
            let bound = self.db.simple(id, kind, 3)?;
            if !boolean(&bound[2])? {
                return Err(unsupported());
            }
            let edge_loop = self.db.simple(reference(&bound[1])?, "EDGE_LOOP", 2)?;
            let uses = list(&edge_loop[1])?;
            if !(3..=16).contains(&uses.len()) {
                return Err(unsupported());
            }
            let mut coedges = Vec::new();
            for value in uses {
                let oriented = self.db.simple(reference(value)?, "ORIENTED_EDGE", 5)?;
                if oriented[1] != Value::Derived || oriented[2] != Value::Derived {
                    return Err(unsupported());
                }
                let edge = self.edge(reference(&oriented[3])?)?;
                let pcurve = self.associations[&edge]
                    .iter()
                    .find(|(id, _, _)| *id == surface_id)
                    .ok_or_else(unsupported)?
                    .1
                    .clone();
                let serialized = self.associations[&edge]
                    .iter()
                    .find(|(id, _, _)| *id == surface_id)
                    .ok_or_else(unsupported)?
                    .2
                    .clone();
                if self
                    .affine
                    .insert((self.solid.shell.faces.len(), edge), serialized)
                    .is_some()
                {
                    return Err(unsupported());
                }
                coedges.push(Coedge {
                    edge,
                    forward: boolean(&oriented[4])?,
                    pcurve,
                });
            }
            if outer.replace(Wire { coedges }).is_some() {
                return Err(unsupported());
            }
        }
        let wires = vec![outer.ok_or_else(unsupported)?];
        self.surfaces.push(surface_id);
        self.solid.shell.faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires,
            orientation: if boolean(&a[3])? { 1 } else { -1 },
        });
        Ok(())
    }
}
/// Decode bounded actual geometry; canonical polygon recognition is separate.
pub(crate) fn read_polygon_solid(input: &str, tol: Tolerance) -> Result<ParsedPolygonStep> {
    Tolerance::new(tol.linear)?;
    let db = Parser::new(input)?.document_polygon_graph()?;
    let root = db.unique("MANIFOLD_SOLID_BREP")?;
    let brep = db.simple(root, "MANIFOLD_SOLID_BREP", 2)?;
    let shell_id = reference(&brep[1])?;
    if db.unique("CLOSED_SHELL")? != shell_id {
        return Err(unsupported());
    }
    let shell = db.simple(shell_id, "CLOSED_SHELL", 2)?;
    let faces = list(&shell[1])?;
    if !(5..=18).contains(&faces.len()) {
        return Err(unsupported());
    }
    let representation = db.unique("ADVANCED_BREP_SHAPE_REPRESENTATION")?;
    let shape = db.simple(representation, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
    if list(&shape[1])? != [Value::Ref(root)] {
        return Err(unsupported());
    }
    let scale = db.units(reference(&shape[2])?)?;
    let mut builder = Builder {
        db: &db,
        scale,
        solid: Solid {
            vertices: vec![],
            edges: vec![],
            shell: Shell { faces: vec![] },
        },
        vertices: BTreeMap::new(),
        edges: BTreeMap::new(),
        associations: BTreeMap::new(),
        surfaces: vec![],
        affine: BTreeMap::new(),
    };
    let mut seen = BTreeSet::new();
    for face in faces {
        let id = reference(face)?;
        if !seen.insert(id) {
            return Err(unsupported());
        }
        builder.face(id)?;
    }
    let n = builder.solid.vertices.len() / 2;
    if !(3..=16).contains(&n)
        || builder.solid.vertices.len() != 2 * n
        || builder.solid.edges.len() != 3 * n
        || builder.solid.shell.faces.len() != n + 2
    {
        return Err(unsupported());
    }
    let mut incidence = vec![(0usize, 0i32); builder.solid.edges.len()];
    let mut attached = vec![BTreeSet::new(); builder.solid.edges.len()];
    for (fi, face) in builder.solid.shell.faces.iter().enumerate() {
        for coedge in &face.wires[0].coedges {
            incidence[coedge.edge].0 += 1;
            incidence[coedge.edge].1 +=
                face.orientation as i32 * if coedge.forward { 1 } else { -1 };
            if !attached[coedge.edge].insert(builder.surfaces[fi]) {
                return Err(unsupported());
            }
        }
    }
    if incidence.iter().any(|v| *v != (2, 0)) {
        return Err(unsupported());
    }
    for (edge, surfaces) in attached.iter().enumerate() {
        let declared: BTreeSet<_> = builder.associations[&edge].iter().map(|p| p.0).collect();
        if surfaces != &declared {
            return Err(unsupported());
        }
    }
    const GEOMETRY: &[&str] = &[
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "LINE",
        "B_SPLINE_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "RATIONAL_B_SPLINE_CURVE",
        "B_SPLINE_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "RATIONAL_B_SPLINE_SURFACE",
        "SURFACE_CURVE",
        "PCURVE",
        "DEFINITIONAL_REPRESENTATION",
        "GEOMETRIC_REPRESENTATION_CONTEXT",
        "AXIS2_PLACEMENT_3D",
        "PLANE",
        "CIRCLE",
        "CYLINDRICAL_SURFACE",
        "SEAM_CURVE",
        "VERTEX_POINT",
        "EDGE_CURVE",
        "ORIENTED_EDGE",
        "EDGE_LOOP",
        "FACE_BOUND",
        "FACE_OUTER_BOUND",
        "ADVANCED_FACE",
        "CLOSED_SHELL",
        "MANIFOLD_SOLID_BREP",
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
    ];
    for (&id, record) in &db.records {
        if record
            .iter()
            .any(|(name, _)| GEOMETRY.contains(&name.as_str()))
            && !db.used.borrow().contains(&id)
        {
            return Err(Error::Unsupported(
                "STEP polygon contains unused or unrepresented geometry",
            ));
        }
    }
    Ok(ParsedPolygonStep {
        solid: builder.solid,
        affine: builder.affine,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Result<NurbsGraphPolygonSolid> {
        let tol = Tolerance::default();
        let source = NurbsGraphSolid::new([80., 60., 20.], 30., tol)?;
        NurbsGraphPolygonSolid::new(
            &source,
            vec![
                [0.15, 0.25],
                [0.65, 0.1],
                [0.9, 0.45],
                [0.65, 0.85],
                [0.2, 0.8],
            ],
            tol,
        )
    }
    #[test]
    fn rational_opt_in_retains_actual_weights_and_raw_affine() -> Result<()> {
        let body = fixture()?;
        let text = body.export_step_mm(Tolerance::default())?;
        assert!(text.contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(Parser::new(&text)?.document_graph().is_err());
        assert!(import_step_mm(&text, Tolerance::default()).is_err());
        let parsed = read_polygon_solid(&text, Tolerance::default())?;
        assert_eq!(parsed.solid.vertices.len(), 10);
        assert_eq!(parsed.solid.edges.len(), 15);
        assert_eq!(parsed.solid.shell.faces.len(), 7);
        assert_eq!(parsed.affine.len(), 30);
        for edge in &parsed.solid.edges {
            let Curve::Nurbs(a) = &edge.curve else {
                unreachable!()
            };
            assert!(body.brep().edges.iter().any(|e|matches!(&e.curve,Curve::Nurbs(b) if a.degree()==b.degree()&&a.knots()==b.knots()&&a.control_points()==b.control_points()&&a.weights()==b.weights())));
        }
        for (fi, face) in parsed.solid.shell.faces.iter().enumerate() {
            let Surface::Nurbs(surface) = &face.surface else {
                unreachable!()
            };
            let canonical=body.brep().shell.faces.iter().find(|f|matches!(&f.surface,Surface::Nurbs(s) if surface.degrees()==s.degrees()&&surface.control_points()==s.control_points()&&surface.weights()==s.weights())).unwrap();
            for c in &face.wires[0].coedges {
                let edge = &parsed.solid.edges[c.edge];
                let original=body.brep().edges.iter().position(|e|matches!((&e.curve,&edge.curve),(Curve::Nurbs(a),Curve::Nurbs(b)) if a.control_points()==b.control_points()&&a.weights()==b.weights())).unwrap();
                let expected = canonical.wires[0]
                    .coedges
                    .iter()
                    .find(|c| c.edge == original)
                    .unwrap();
                let PCurve::Affine { origin, direction } = expected.pcurve else {
                    unreachable!()
                };
                let (ratios, magnitude) =
                    crate::nurbs_graph_step::decompose_affine(origin, direction)?;
                assert_eq!(
                    parsed.affine[&(fi, c.edge)],
                    SerializedAffine {
                        origin,
                        direction: ratios,
                        magnitude
                    }
                );
            }
        }
        Ok(())
    }
    #[test]
    fn malformed_complex_sets_nonpositive_weights_and_unused_geometry_reject() -> Result<()> {
        let text = fixture()?.export_step_mm(Tolerance::default())?;
        let missing = text.replacen("BOUNDED_CURVE()", "", 1);
        assert!(Parser::new(&missing)?.document_polygon_graph().is_err());
        let duplicate = text.replacen("BOUNDED_CURVE()", "BOUNDED_CURVE() BOUNDED_CURVE()", 1);
        assert!(Parser::new(&duplicate)?.document_polygon_graph().is_err());
        let start =
            text.find("RATIONAL_B_SPLINE_CURVE((").unwrap() + "RATIONAL_B_SPLINE_CURVE((".len();
        let end = start + text[start..].find(',').unwrap();
        for weight in ["0.", "-1."] {
            let mut bad = text.clone();
            bad.replace_range(start..end, weight);
            assert!(read_polygon_solid(&bad, Tolerance::default()).is_err());
        }
        let unused = text.replacen(
            "ENDSEC;\nEND-ISO-10303-21;",
            "#999999=CARTESIAN_POINT('',(0.,0.,0.));\nENDSEC;\nEND-ISO-10303-21;",
            1,
        );
        assert_ne!(unused, text);
        assert!(read_polygon_solid(&unused, Tolerance::default()).is_err());
        Ok(())
    }
}
