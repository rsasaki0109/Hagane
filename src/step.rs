//! Original ISO 10303-21 / AP214 writer for one planar straight-edge solid.
use crate::*;
use std::fmt::Write;
#[derive(Default)]
struct Writer {
    records: Vec<String>,
}
impl Writer {
    fn entity(&mut self, body: String) -> usize {
        self.records.push(body);
        self.records.len()
    }
    fn point(&mut self, p: Point3) -> usize {
        self.entity(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            number(p.x),
            number(p.y),
            number(p.z)
        ))
    }
    fn direction(&mut self, v: Vec3) -> usize {
        self.entity(format!(
            "DIRECTION('',({},{},{}))",
            number(v.x),
            number(v.y),
            number(v.z)
        ))
    }
}
fn number(value: f64) -> String {
    let s = value.to_string();
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s.replace('e', "E")
    } else {
        format!("{s}.")
    }
}
fn refs(ids: &[usize]) -> String {
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn logical(value: bool) -> &'static str {
    if value {
        ".T."
    } else {
        ".F."
    }
}
/// Export one validated closed planar straight-edge B-rep in millimetres.
/// One AP214 MANIFOLD_SOLID_BREP preserves shared vertices/edges, face bounds
/// and surface orientation. Curved edges/surfaces are explicitly unsupported.
/// Coordinates and tolerance are interpreted as mm; no implicit unit conversion.
pub fn export_step_planar_mm(solid: &Solid, tolerance: Tolerance) -> Result<String> {
    solid.validate(tolerance)?;
    if solid.vertices.len() > 4096 || solid.edges.len() > 4096 || solid.shell.faces.len() > 512 {
        return Err(Error::Unsupported(
            "STEP planar export supports at most 4096 vertices/edges and 512 faces",
        ));
    }
    if solid
        .edges
        .iter()
        .any(|e| !matches!(e.curve, Curve::Line { .. }))
        || solid
            .shell
            .faces
            .iter()
            .any(|f| !matches!(f.surface, Surface::Plane { .. }))
    {
        return Err(Error::Unsupported(
            "STEP export currently requires planar surfaces and straight edges",
        ));
    }
    let mut w = Writer::default();
    let points: Vec<_> = solid.vertices.iter().map(|v| w.point(v.point)).collect();
    let vertices: Vec<_> = points
        .iter()
        .map(|id| w.entity(format!("VERTEX_POINT('',#{id})")))
        .collect();
    let mut edges = Vec::new();
    for e in &solid.edges {
        let Curve::Line { a, b } = e.curve else {
            unreachable!()
        };
        let delta = b - a;
        let length = delta.norm();
        if !length.is_finite() || length <= tolerance.linear {
            return Err(Error::InvalidInput("STEP line length is unresolved"));
        }
        let point = w.point(a);
        let dir = w.direction(delta * (1. / length));
        let vector = w.entity(format!("VECTOR('',#{dir},{})", number(length)));
        let line = w.entity(format!("LINE('',#{point},#{vector})"));
        edges.push(w.entity(format!(
            "EDGE_CURVE('',#{},#{},#{line},.T.)",
            vertices[e.vertices[0]], vertices[e.vertices[1]]
        )));
    }
    let mut faces = Vec::new();
    for face in &solid.shell.faces {
        let Surface::Plane { origin, u, v } = face.surface else {
            unreachable!()
        };
        let normal = u.cross(v);
        let point = w.point(origin);
        let axis = w.direction(normal);
        let reference = w.direction(u);
        let placement = w.entity(format!(
            "AXIS2_PLACEMENT_3D('',#{point},#{axis},#{reference})"
        ));
        let plane = w.entity(format!("PLANE('',#{placement})"));
        let mut bounds = Vec::new();
        for (i, wire) in face.wires.iter().enumerate() {
            let oriented: Vec<_> = wire
                .coedges
                .iter()
                .map(|c| {
                    w.entity(format!(
                        "ORIENTED_EDGE('',*,*,#{},{})",
                        edges[c.edge],
                        logical(c.forward)
                    ))
                })
                .collect();
            let edge_loop = w.entity(format!("EDGE_LOOP('',({}))", refs(&oriented)));
            bounds.push(w.entity(format!(
                "{}('',#{edge_loop},.T.)",
                if i == 0 {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                }
            )));
        }
        faces.push(w.entity(format!(
            "ADVANCED_FACE('',({}),#{plane},{})",
            refs(&bounds),
            logical(face.orientation > 0)
        )));
    }
    let shell = w.entity(format!("CLOSED_SHELL('',({}))", refs(&faces)));
    let brep = w.entity(format!("MANIFOLD_SOLID_BREP('Hagane part',#{shell})"));
    let length_unit = w.entity("(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))".into());
    let angle_unit = w.entity("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))".into());
    let solid_angle = w.entity("(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())".into());
    let uncertainty=w.entity(format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({}),#{length_unit},'distance_accuracy_value','')",number(tolerance.linear)));
    let context=w.entity(format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{uncertainty})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length_unit},#{angle_unit},#{solid_angle})) REPRESENTATION_CONTEXT('',''))"));
    let representation = w.entity(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{brep}),#{context})"
    ));
    let application = w.entity("APPLICATION_CONTEXT('automotive_design')".into());
    w.entity(format!("APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,#{application})"));
    let product_context = w.entity(format!("PRODUCT_CONTEXT('',#{application},'mechanical')"));
    let product = w.entity(format!(
        "PRODUCT('hagane','Hagane part','',(#{product_context}))"
    ));
    let formation = w.entity(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"));
    let definition_context = w.entity(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{application},'design')"
    ));
    let definition = w.entity(format!(
        "PRODUCT_DEFINITION('design','',#{formation},#{definition_context})"
    ));
    let shape = w.entity(format!("PRODUCT_DEFINITION_SHAPE('','',#{definition})"));
    w.entity(format!(
        "SHAPE_DEFINITION_REPRESENTATION(#{shape},#{representation})"
    ));
    let mut result=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Hagane exact planar B-rep'),'2;1');\nFILE_NAME('hagane.step','',('Hagane'),(''),'Hagane','Hagane','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n");
    for (i, record) in w.records.iter().enumerate() {
        writeln!(result, "#{}={record};", i + 1).expect("writing into String");
    }
    result.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(result)
}
/// Rebuild a bounded versioned workflow and export supported planar geometry.
/// Export failures do not affect an incremental workflow session.
pub fn export_workflow_step_mm_json(input: &str) -> Result<String> {
    if input.len() > 65536 {
        return Err(Error::InvalidInput("STEP workflow input exceeds 64 KiB"));
    }
    let doc: WorkflowDocument = serde_json::from_str(input)
        .map_err(|_| Error::InvalidInput("invalid STEP workflow document"))?;
    let solid = doc
        .rebuild()
        .map_err(|_| Error::InvalidInput("STEP workflow geometry was rejected"))?;
    let step = export_step_planar_mm(&solid, Tolerance::new(doc.tolerance.linear)?)?;
    serde_json::to_string(&serde_json::json!({"step":step,"units":"mm","schema":"AP214","faces":solid.shell.faces.len()})).map_err(|_|Error::InvalidInput("STEP export report serialization failed"))
}
