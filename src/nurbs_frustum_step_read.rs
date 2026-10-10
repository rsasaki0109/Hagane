//! Isolated bounded raw decoding for canonical unplaced rational frusta.
use crate::step_read::{boolean, list, number, reference, Database, Parser, Value};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerializedAffine {
    pub origin: [f64; 2],
    pub direction: [f64; 2],
    pub magnitude: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerializedLine {
    pub origin: Point3,
    pub direction: Vec3,
    pub magnitude: f64,
}
pub(crate) struct ParsedFrustumStep {
    pub solid: Solid,
    pub affine: BTreeMap<(usize, usize), SerializedAffine>,
    pub lines: BTreeMap<usize, SerializedLine>,
}
fn unsupported() -> Error {
    Error::Unsupported(
        "STEP frustum requires exact supported rational and analytic geometry with shared topology",
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
fn curve(db: &Database, id: u32, scale: f64, dimensions: usize) -> Result<NurbsCurve> {
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
        .map(|p| {
            let a = db.simple(reference(p)?, "CARTESIAN_POINT", 2)?;
            let v = list(&a[1])?;
            if v.len() != dimensions {
                return Err(unsupported());
            }
            Ok(Point3::new(
                number(&v[0])? * scale,
                number(&v[1])? * scale,
                if dimensions == 3 {
                    number(&v[2])? * scale
                } else {
                    0.
                },
            ))
        })
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
fn coordinates3(db: &Database, id: u32, kind: &str) -> Result<Vec3> {
    let a = db.simple(id, kind, 2)?;
    let v = list(&a[1])?;
    if v.len() != 3 {
        return Err(unsupported());
    }
    let result = Vec3::new(number(&v[0])?, number(&v[1])?, number(&v[2])?);
    if !result.finite() {
        return Err(unsupported());
    }
    Ok(result)
}
fn pcurve(db: &Database, id: u32) -> Result<(u32, PCurve, Option<SerializedAffine>)> {
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
    let geometry = reference(&items[0])?;
    if !db
        .records
        .get(&geometry)
        .ok_or_else(unsupported)?
        .iter()
        .any(|(n, _)| n == "LINE")
    {
        return Ok((
            surface,
            PCurve::Nurbs(Box::new(curve(db, geometry, 1., 2)?)),
            None,
        ));
    }
    let line = db.simple(geometry, "LINE", 3)?;
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
        Some(SerializedAffine {
            origin,
            direction,
            magnitude: length,
        }),
    ))
}
type Association = (u32, PCurve, Option<SerializedAffine>);
struct Builder<'a> {
    db: &'a Database,
    scale: f64,
    solid: Solid,
    vertices: BTreeMap<u32, usize>,
    edges: BTreeMap<u32, usize>,
    associations: BTreeMap<usize, Vec<Association>>,
    surfaces: Vec<u32>,
    affine: BTreeMap<(usize, usize), SerializedAffine>,
    lines: BTreeMap<usize, SerializedLine>,
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
        let id3 = reference(&geometry[1])?;
        let (curve, rawline) = if self.db.records[&id3].iter().any(|(name, _)| name == "LINE") {
            let a = self.db.simple(id3, "LINE", 3)?;
            let origin = self.db.point(reference(&a[1])?, self.scale)?;
            let vector = self.db.simple(reference(&a[2])?, "VECTOR", 3)?;
            let direction = coordinates3(self.db, reference(&vector[1])?, "DIRECTION")?;
            let magnitude = number(&vector[2])?;
            if !magnitude.is_finite() || magnitude <= 0. || direction == Vec3::new(0., 0., 0.) {
                return Err(unsupported());
            }
            let endpoint = origin + direction * magnitude;
            if !endpoint.finite() {
                return Err(unsupported());
            }
            (
                Curve::Line {
                    a: origin,
                    b: endpoint,
                },
                Some(SerializedLine {
                    origin,
                    direction,
                    magnitude,
                }),
            )
        } else {
            (
                Curve::Nurbs(Box::new(curve(self.db, id3, self.scale, 3)?)),
                None,
            )
        };
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
        self.solid.edges.push(Edge { vertices, curve });
        if let Some(raw) = rawline {
            self.lines.insert(index, raw);
        }
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
        let surface = if self.db.records[&surface_id]
            .iter()
            .any(|(name, _)| name == "PLANE")
        {
            let plane = self.db.simple(surface_id, "PLANE", 2)?;
            let placement = self
                .db
                .simple(reference(&plane[1])?, "AXIS2_PLACEMENT_3D", 4)?;
            let origin = self.db.point(reference(&placement[1])?, self.scale)?;
            let axis = coordinates3(self.db, reference(&placement[2])?, "DIRECTION")?;
            let u = coordinates3(self.db, reference(&placement[3])?, "DIRECTION")?;
            if axis != Vec3::new(0., 0., 1.) || u != Vec3::new(1., 0., 0.) {
                return Err(unsupported());
            }
            Surface::Plane {
                origin,
                u,
                v: Vec3::new(0., 1., 0.),
            }
        } else {
            Surface::Nurbs(Box::new(surface(self.db, surface_id, self.scale)?))
        };
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
                if let Some(serialized) = serialized {
                    if self
                        .affine
                        .insert((self.solid.shell.faces.len(), edge), serialized)
                        .is_some()
                    {
                        return Err(unsupported());
                    }
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
            surface,
            wires,
            orientation: if boolean(&a[3])? { 1 } else { -1 },
        });
        Ok(())
    }
}
/// Decode bounded actual geometry; canonical frustum recognition is separate.
pub(crate) fn read_frustum_solid(input: &str, tol: Tolerance) -> Result<ParsedFrustumStep> {
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
    if faces.len() != 6 {
        return Err(unsupported());
    }
    let representation = db.unique("ADVANCED_BREP_SHAPE_REPRESENTATION")?;
    let shape = db.simple(representation, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
    if list(&shape[1])? != [Value::Ref(root)] {
        return Err(unsupported());
    }
    let scale = db.units(reference(&shape[2])?)?;
    if scale != 1. {
        return Err(Error::Unsupported(
            "frustum STEP import supports millimetres only",
        ));
    }
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
        lines: BTreeMap::new(),
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
    if n != 4
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
                "STEP frustum contains unused or unrepresented geometry",
            ));
        }
    }
    Ok(ParsedFrustumStep {
        solid: builder.solid,
        affine: builder.affine,
        lines: builder.lines,
    })
}
