//! Exact, narrow AP214 recognition of unplaced full-domain polynomial graph bodies.
use crate::step_read::{boolean, list, number, reference, Database, Parser, Value};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
fn unsupported() -> Error {
    Error::Unsupported("STEP NURBS graph import requires an exact unplaced full-domain canonical six-face graph solid")
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
    let a = db.simple(id, "B_SPLINE_CURVE_WITH_KNOTS", 9)?;
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
    let weights = vec![1.; points.len()];
    let knots = knots(&a[6], &a[7], degree, points.len())?;
    NurbsCurve::new(degree, knots, points, weights)
}
fn surface(db: &Database, id: u32, scale: f64) -> Result<NurbsSurface> {
    let a = db.simple(id, "B_SPLINE_SURFACE_WITH_KNOTS", 13)?;
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
    let weights = vec![1.; points.len()];
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
fn pcurve(db: &Database, id: u32) -> Result<(u32, PCurve)> {
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
    // Scoped graph PCurves have exact unit coordinate-axis directions. Never
    // normalize an approximate direction or silently change its parameter.
    if magnitude != 1. || !length.is_finite() || length <= 0. {
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
    ))
}
struct Builder<'a> {
    db: &'a Database,
    scale: f64,
    solid: Solid,
    vertices: BTreeMap<u32, usize>,
    edges: BTreeMap<u32, usize>,
    associations: BTreeMap<usize, Vec<(u32, PCurve)>>,
    surfaces: Vec<u32>,
}
impl Builder<'_> {
    fn vertex(&mut self, id: u32) -> Result<usize> {
        if let Some(&v) = self.vertices.get(&id) {
            return Ok(v);
        }
        if self.solid.vertices.len() >= 8 {
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
        if self.solid.edges.len() >= 12 {
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
        if bounds.len() != 1 {
            return Err(unsupported());
        }
        let surface_id = reference(&a[2])?;
        if self.surfaces.contains(&surface_id) {
            return Err(unsupported());
        }
        let surface = surface(self.db, surface_id, self.scale)?;
        let bound = self
            .db
            .simple(reference(&bounds[0])?, "FACE_OUTER_BOUND", 3)?;
        if !boolean(&bound[2])? {
            return Err(unsupported());
        }
        let edge_loop = self.db.simple(reference(&bound[1])?, "EDGE_LOOP", 2)?;
        let uses = list(&edge_loop[1])?;
        if uses.len() != 4 {
            return Err(unsupported());
        }
        let mut coedges = Vec::new();
        for value in uses {
            let oriented = self.db.simple(reference(value)?, "ORIENTED_EDGE", 5)?;
            if oriented[1] != Value::Derived || oriented[2] != Value::Derived {
                return Err(unsupported());
            }
            let edge = self.edge(reference(&oriented[3])?)?;
            let associations = &self.associations[&edge];
            let pcurve = associations
                .iter()
                .find(|(id, _)| *id == surface_id)
                .ok_or_else(unsupported)?
                .1
                .clone();
            coedges.push(Coedge {
                edge,
                forward: boolean(&oriented[4])?,
                pcurve,
            });
        }
        self.surfaces.push(surface_id);
        self.solid.shell.faces.push(Face {
            surface: Surface::Nurbs(Box::new(surface)),
            wires: vec![Wire { coedges }],
            orientation: if boolean(&a[3])? { 1 } else { -1 },
        });
        Ok(())
    }
}
fn same_curve(a: &Curve, b: &Curve) -> bool {
    let (Curve::Nurbs(a), Curve::Nurbs(b)) = (a, b) else {
        return false;
    };
    a.degree() == b.degree()
        && a.knots() == b.knots()
        && a.weights() == b.weights()
        && a.control_points() == b.control_points()
}
fn same_surface(a: &Surface, b: &Surface) -> Result<bool> {
    let (Surface::Nurbs(a), Surface::Nurbs(b)) = (a, b) else {
        return Ok(false);
    };
    Ok(a.degrees() == b.degrees()
        && a.control_counts() == b.control_counts()
        && a.knots(0)? == b.knots(0)?
        && a.knots(1)? == b.knots(1)?
        && a.weights() == b.weights()
        && a.control_points() == b.control_points())
}
fn recognize(mut actual: Solid, tol: Tolerance) -> Result<NurbsGraphSolid> {
    if actual.vertices.len() != 8 || actual.edges.len() != 12 || actual.shell.faces.len() != 6 {
        return Err(unsupported());
    }
    let mut candidates = Vec::new();
    for face in &actual.shell.faces {
        let Surface::Nurbs(s) = &face.surface else {
            return Err(unsupported());
        };
        if s.degrees() != [2, 2]
            || s.control_counts() != [3, 3]
            || s.domain() != [[0., 1.], [0., 1.]]
        {
            continue;
        }
        let p = s.control_points();
        let l = p[6].x;
        let w = p[2].y;
        let h = p[0].z;
        let b = p[4].z - h;
        if let Ok(candidate) = NurbsGraphSolid::new([l, w, h], b, tol) {
            if same_surface(&face.surface, &candidate.brep().shell.faces[1].surface)? {
                candidates.push(candidate);
            }
        }
    }
    if candidates.len() != 1 {
        return Err(unsupported());
    }
    let mut candidate = candidates.pop().unwrap();
    let expected = candidate.brep();
    let mut vertex_map = Vec::new();
    let mut seen = BTreeSet::new();
    for vertex in &actual.vertices {
        let index = expected
            .vertices
            .iter()
            .position(|v| v.point == vertex.point)
            .ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        vertex_map.push(index);
    }
    let mut vertices = expected.vertices.clone();
    for (index, vertex) in actual.vertices.into_iter().enumerate() {
        vertices[vertex_map[index]] = vertex;
    }
    actual.vertices = vertices;
    let mut edge_map = Vec::new();
    seen.clear();
    for edge in &mut actual.edges {
        edge.vertices = edge.vertices.map(|i| vertex_map[i]);
        let index = expected
            .edges
            .iter()
            .position(|e| e.vertices == edge.vertices && same_curve(&e.curve, &edge.curve))
            .ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        edge_map.push(index);
    }
    let mut edges = expected.edges.clone();
    for (index, edge) in actual.edges.into_iter().enumerate() {
        edges[edge_map[index]] = edge;
    }
    actual.edges = edges;
    let mut faces = expected.shell.faces.clone();
    seen.clear();
    for mut face in actual.shell.faces {
        let mut matching = None;
        for (index, e) in expected.shell.faces.iter().enumerate() {
            if same_surface(&e.surface, &face.surface)? {
                matching = Some(index);
                break;
            }
        }
        let index = matching.ok_or_else(unsupported)?;
        if !seen.insert(index) {
            return Err(unsupported());
        }
        for c in &mut face.wires[0].coedges {
            c.edge = edge_map[c.edge];
        }
        // A cyclic loop start is not geometric data; retain traversal and reindex it.
        let target = expected.shell.faces[index].wires[0].coedges[0].edge;
        let start = face.wires[0]
            .coedges
            .iter()
            .position(|c| c.edge == target)
            .ok_or_else(unsupported)?;
        face.wires[0].coedges.rotate_left(start);
        faces[index] = face;
    }
    actual.shell.faces = faces;
    candidate.solid = actual;
    candidate.validate(tol).map_err(|_| unsupported())?;
    Ok(candidate)
}
/// Import one exact full-domain, unplaced canonical polynomial graph from AP214.
/// Metre or millimetre coordinates become mm; knots and UV parameters are unchanged.
/// Placement, trimming, holes, rational weights and approximate recognition are unsupported.
pub fn import_step_nurbs_graph_mm(input: &str, tol: Tolerance) -> Result<NurbsGraphSolid> {
    Tolerance::new(tol.linear)?;
    let db = Parser::new(input)?.document_graph()?;
    let root = db.unique("MANIFOLD_SOLID_BREP")?;
    let brep = db.simple(root, "MANIFOLD_SOLID_BREP", 2)?;
    let shell = reference(&brep[1])?;
    if db.unique("CLOSED_SHELL")? != shell {
        return Err(unsupported());
    }
    let shell = db.simple(shell, "CLOSED_SHELL", 2)?;
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
    let mut builder = Builder {
        db: &db,
        scale,
        solid: Solid {
            vertices: Vec::new(),
            edges: Vec::new(),
            shell: Shell { faces: Vec::new() },
        },
        vertices: BTreeMap::new(),
        edges: BTreeMap::new(),
        associations: BTreeMap::new(),
        surfaces: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for face in faces {
        let id = reference(face)?;
        if !seen.insert(id) {
            return Err(unsupported());
        }
        builder.face(id)?;
    }
    const GEOMETRY: &[&str] = &[
        "CARTESIAN_POINT",
        "DIRECTION",
        "VECTOR",
        "LINE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "B_SPLINE_SURFACE_WITH_KNOTS",
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
                "STEP graph contains unused or unrepresented geometry",
            ));
        }
    }
    recognize(builder.solid, tol)
}
