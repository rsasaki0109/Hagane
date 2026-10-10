//! Original ISO 10303-21 / AP214 writer for planar and full cylindrical solids.
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
    fn placement(&mut self, origin: Point3, normal: Vec3, reference: Vec3) -> usize {
        let point = self.point(origin);
        let axis = self.direction(normal);
        let reference = self.direction(reference);
        self.entity(format!(
            "AXIS2_PLACEMENT_3D('',#{point},#{axis},#{reference})"
        ))
    }
    fn analytic_pcurve(
        &mut self,
        surface: usize,
        pcurve: &PCurve,
        context: usize,
    ) -> Result<usize> {
        let (center, radius, phase) = match *pcurve {
            PCurve::Affine { .. } => return self.seam_pcurve(surface, pcurve, context),
            PCurve::Circle { center, radius } => (center, radius, 0.),
            PCurve::Arc {
                center,
                radius,
                start_angle,
                ..
            } => (center, radius, start_angle),
            _ => {
                return Err(Error::Unsupported(
                    "STEP bounded analytic pcurve is unsupported",
                ))
            }
        };
        if !center.iter().all(|v| v.is_finite())
            || !radius.is_finite()
            || radius <= 0.
            || !phase.is_finite()
        {
            return Err(Error::InvalidTopology("STEP UV circle is invalid"));
        }
        let point = self.entity(format!(
            "CARTESIAN_POINT('',({},{}))",
            number(center[0]),
            number(center[1])
        ));
        let direction = self.entity(format!(
            "DIRECTION('',({},{}))",
            number(phase.cos()),
            number(phase.sin())
        ));
        let placement = self.entity(format!("AXIS2_PLACEMENT_2D('',#{point},#{direction})"));
        let circle = self.entity(format!("CIRCLE('',#{placement},{})", number(radius)));
        let representation = self.entity(format!(
            "DEFINITIONAL_REPRESENTATION('',(#{circle}),#{context})"
        ));
        Ok(self.entity(format!("PCURVE('',#{surface},#{representation})")))
    }
    fn seam_pcurve(&mut self, surface: usize, pcurve: &PCurve, context: usize) -> Result<usize> {
        let PCurve::Affine { origin, direction } = *pcurve else {
            return Err(Error::Unsupported(
                "STEP cylinder seam requires an affine pcurve",
            ));
        };
        let length = direction[0].hypot(direction[1]);
        if !length.is_finite() || length <= 0.0 {
            return Err(Error::InvalidTopology("STEP seam pcurve is degenerate"));
        }
        let point = self.entity(format!(
            "CARTESIAN_POINT('',({},{}))",
            number(origin[0]),
            number(origin[1])
        ));
        let direction = self.entity(format!(
            "DIRECTION('',({},{}))",
            number(direction[0] / length),
            number(direction[1] / length)
        ));
        let vector = self.entity(format!("VECTOR('',#{direction},{})", number(length)));
        let line = self.entity(format!("LINE('',#{point},#{vector})"));
        let representation = self.entity(format!(
            "DEFINITIONAL_REPRESENTATION('',(#{line}),#{context})"
        ));
        Ok(self.entity(format!("PCURVE('',#{surface},#{representation})")))
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
    write_step(solid, tolerance, true, false)
}
/// Export one validated planar/full cylindrical B-rep in millimetres.
/// Supports lines and complete circles, planar faces and rectangular 2π cylinder
/// faces. Periodic seams retain both UV uses through SEAM_CURVE/PCURVE entities.
/// Bounded arcs, ellipses, skew circular surfaces and height-graph trims are
/// explicitly unsupported; there is no mesh or tolerance-offset approximation.
pub fn export_step_mm(solid: &Solid, tolerance: Tolerance) -> Result<String> {
    write_step(solid, tolerance, false, false)
}
/// Opt-in bounded analytic export, retaining both actual UV uses of each edge.
/// Supports canonical positive circular arcs with sweep at most pi, and
/// rectangular cylinder trims. Negative and long arcs remain unsupported.
pub fn export_step_bounded_analytic_mm(solid: &Solid, linear_tolerance: f64) -> Result<String> {
    write_step(solid, Tolerance::new(linear_tolerance)?, false, true)
}
fn write_step(
    solid: &Solid,
    tolerance: Tolerance,
    planar_only: bool,
    bounded: bool,
) -> Result<String> {
    solid.validate(tolerance)?;
    if solid.vertices.len() > 4096 || solid.edges.len() > 4096 || solid.shell.faces.len() > 512 {
        return Err(Error::Unsupported(
            "STEP export supports at most 4096 vertices/edges and 512 faces",
        ));
    }
    if planar_only
        && (solid
            .edges
            .iter()
            .any(|e| !matches!(e.curve, Curve::Line { .. }))
            || solid
                .shell
                .faces
                .iter()
                .any(|f| !matches!(f.surface, Surface::Plane { .. })))
    {
        return Err(Error::Unsupported(
            "STEP export currently requires planar surfaces and straight edges",
        ));
    }
    for edge in &solid.edges {
        if !matches!(
            edge.curve,
            Curve::Line { .. } | Curve::Circle { .. } | Curve::FramedCircle { .. }
        ) && !(bounded && matches!(edge.curve, Curve::Arc { .. }))
        {
            return Err(Error::Unsupported(if bounded {
                "bounded STEP supports lines, complete circles and canonical bounded arcs only"
            } else {
                "STEP export supports lines and complete circles only"
            }));
        }
    }
    let mut bounded_arithmetic = 0.;
    if bounded {
        let mut world = solid.vertices.iter().fold(0_f64, |m, v| {
            m.max(v.point.x.abs())
                .max(v.point.y.abs())
                .max(v.point.z.abs())
        });
        for edge in &solid.edges {
            if let Curve::Arc { frame, .. } | Curve::FramedCircle { frame, .. } = edge.curve {
                let p = frame.origin();
                world = world.max(p.x.abs()).max(p.y.abs()).max(p.z.abs());
            }
        }
        for face in &solid.shell.faces {
            let origin = match face.surface {
                Surface::Plane { origin, .. } => origin,
                Surface::Cylinder { center, .. } => center,
                Surface::FramedCylinder { frame, .. } => frame.origin(),
                _ => {
                    return Err(Error::Unsupported(
                        "bounded STEP supports planes and rectangular cylinders only",
                    ))
                }
            };
            world = world
                .max(origin.x.abs())
                .max(origin.y.abs())
                .max(origin.z.abs());
        }
        let arithmetic = 4096. * f64::EPSILON * world;
        bounded_arithmetic = arithmetic;
        if !arithmetic.is_finite() || arithmetic >= tolerance.linear / 4. {
            return Err(Error::Unsupported(
                "bounded STEP coordinate precision is unresolved",
            ));
        }
        for edge in &solid.edges {
            if let Curve::Arc { sweep, .. } = edge.curve {
                let a = solid.vertices[edge.vertices[0]].point;
                let b = solid.vertices[edge.vertices[1]].point;
                if !sweep.is_finite()
                    || sweep <= 0.
                    || sweep > std::f64::consts::PI
                    || edge.vertices[0] == edge.vertices[1]
                    || (a - b).norm() <= 10. * tolerance.linear + 8. * arithmetic
                {
                    return Err(Error::Unsupported(
                        "STEP bounded arc endpoints or sweep are unresolved",
                    ));
                }
            }
        }
        for face in &solid.shell.faces {
            for coedge in face.wires.iter().flat_map(|w| &w.coedges) {
                if let PCurve::Arc {
                    start_angle,
                    radius,
                    ..
                } = coedge.pcurve
                {
                    if 128. * f64::EPSILON * start_angle.abs().max(1.) * radius
                        >= tolerance.linear / 4.
                    {
                        return Err(Error::Unsupported(
                            "bounded STEP UV circle phase is unresolved",
                        ));
                    }
                }
                if !matches!(
                    coedge.pcurve,
                    PCurve::Affine { .. } | PCurve::Circle { .. } | PCurve::Arc { .. }
                ) {
                    return Err(Error::Unsupported(
                        "STEP bounded analytic pcurve is unsupported",
                    ));
                }
            }
        }
    }
    for face in &solid.shell.faces {
        match face.surface {
            Surface::Plane { .. } => {}
            Surface::Cylinder { .. } | Surface::FramedCylinder { .. } => {
                let span = face.cylinder_span()?;
                if bounded && (!span.is_finite() || span <= 0. || span > std::f64::consts::TAU) {
                    return Err(Error::Unsupported(
                        "bounded STEP requires a resolved rectangular cylinder trim",
                    ));
                }
                if bounded {
                    let radius = match face.surface {
                        Surface::Cylinder { radius, .. }
                        | Surface::FramedCylinder { radius, .. } => radius,
                        _ => unreachable!(),
                    };
                    let gap = 10. * tolerance.linear + 8. * bounded_arithmetic;
                    if !(radius * span).is_finite()
                        || radius * span <= gap
                        || (span < std::f64::consts::TAU
                            && radius * (std::f64::consts::TAU - span) <= gap)
                    {
                        return Err(Error::Unsupported(
                            "bounded STEP cylinder angular endpoints are unresolved",
                        ));
                    }
                }
                if !bounded && span != std::f64::consts::TAU {
                    return Err(Error::Unsupported(
                        "STEP export requires a full rectangular cylinder trim",
                    ));
                }
            }
            _ => {
                return Err(Error::Unsupported(if bounded {
                    "bounded STEP supports planes and rectangular cylinders only"
                } else {
                    "STEP export supports planes and full rectangular cylinders only"
                }))
            }
        }
    }
    let mut w = Writer::default();
    let points: Vec<_> = solid.vertices.iter().map(|v| w.point(v.point)).collect();
    let vertices: Vec<_> = points
        .iter()
        .map(|id| w.entity(format!("VERTEX_POINT('',#{id})")))
        .collect();
    let mut surfaces = Vec::new();
    for face in &solid.shell.faces {
        let (name, origin, normal, reference, radius) = match face.surface {
            Surface::Plane { origin, u, v } => ("PLANE", origin, u.cross(v), u, None),
            Surface::Cylinder { center, radius, .. } => (
                "CYLINDRICAL_SURFACE",
                center,
                Vec3::new(0., 0., 1.),
                Vec3::new(1., 0., 0.),
                Some(radius),
            ),
            Surface::FramedCylinder { frame, radius, .. } => (
                "CYLINDRICAL_SURFACE",
                frame.origin(),
                frame.axes()[2],
                frame.axes()[0],
                Some(radius),
            ),
            _ => unreachable!(),
        };
        let placement = w.placement(origin, normal, reference);
        surfaces.push(w.entity(match radius {
            Some(radius) => format!("{name}('',#{placement},{})", number(radius)),
            None => format!("{name}('',#{placement})"),
        }));
    }
    let mut uses = vec![Vec::new(); solid.edges.len()];
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        for wire in &face.wires {
            for coedge in &wire.coedges {
                uses[coedge.edge].push((fi, coedge));
            }
        }
    }
    let seam_context = if bounded || uses.iter().any(|u| u.len() == 2 && u[0].0 == u[1].0) {
        Some(w.entity("(GEOMETRIC_REPRESENTATION_CONTEXT(2) REPRESENTATION_CONTEXT('',''))".into()))
    } else {
        None
    };
    let mut edges = Vec::new();
    for (ei, e) in solid.edges.iter().enumerate() {
        let mut geometry = match e.curve {
            Curve::Line { a, b } => {
                let delta = b - a;
                let length = delta.norm();
                if !length.is_finite() || length <= tolerance.linear {
                    return Err(Error::InvalidInput("STEP line length is unresolved"));
                }
                let point = w.point(a);
                let dir = w.direction(delta * (1. / length));
                let vector = w.entity(format!("VECTOR('',#{dir},{})", number(length)));
                w.entity(format!("LINE('',#{point},#{vector})"))
            }
            Curve::Circle { center, radius } => {
                let placement = w.placement(center, Vec3::new(0., 0., 1.), Vec3::new(1., 0., 0.));
                w.entity(format!("CIRCLE('',#{placement},{})", number(radius)))
            }
            Curve::FramedCircle { frame, radius } | Curve::Arc { frame, radius, .. } => {
                let placement = w.placement(frame.origin(), frame.axes()[2], frame.axes()[0]);
                w.entity(format!("CIRCLE('',#{placement},{})", number(radius)))
            }
            _ => unreachable!(),
        };
        let edge_uses = &uses[ei];
        if !bounded && edge_uses.len() == 2 && edge_uses[0].0 == edge_uses[1].0 {
            let fi = edge_uses[0].0;
            if !matches!(
                solid.shell.faces[fi].surface,
                Surface::Cylinder { .. } | Surface::FramedCylinder { .. }
            ) || !matches!(e.curve, Curve::Line { .. })
            {
                return Err(Error::Unsupported(
                    "STEP periodic seam requires a cylindrical line",
                ));
            }
            let pcurves = edge_uses
                .iter()
                .map(|(_, c)| w.seam_pcurve(surfaces[fi], &c.pcurve, seam_context.unwrap()))
                .collect::<Result<Vec<_>>>()?;
            geometry = w.entity(format!(
                "SEAM_CURVE('',#{geometry},({}),.CURVE_3D.)",
                refs(&pcurves)
            ));
        }
        if bounded {
            let pcurves = edge_uses
                .iter()
                .map(|(fi, c)| w.analytic_pcurve(surfaces[*fi], &c.pcurve, seam_context.unwrap()))
                .collect::<Result<Vec<_>>>()?;
            let kind = if edge_uses.len() == 2 && edge_uses[0].0 == edge_uses[1].0 {
                "SEAM_CURVE"
            } else {
                "SURFACE_CURVE"
            };
            geometry = w.entity(format!(
                "{kind}('',#{geometry},({}),.CURVE_3D.)",
                refs(&pcurves)
            ));
        }
        let sense = if bounded {
            logical(!matches!(e.curve, Curve::Arc { sweep, .. } if sweep < 0.))
        } else {
            ".T."
        };
        edges.push(w.entity(format!(
            "EDGE_CURVE('',#{},#{},#{geometry},{sense})",
            vertices[e.vertices[0]], vertices[e.vertices[1]]
        )));
    }
    let mut faces = Vec::new();
    for (fi, face) in solid.shell.faces.iter().enumerate() {
        let surface = surfaces[fi];
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
            "ADVANCED_FACE('',({}),#{surface},{})",
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
    let mut result=String::from("ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('Hagane exact B-rep'),'2;1');\nFILE_NAME('hagane.step','',('Hagane'),(''),'Hagane','Hagane','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n");
    for (i, record) in w.records.iter().enumerate() {
        writeln!(result, "#{}={record};", i + 1).expect("writing into String");
    }
    result.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(result)
}
/// Rebuild a bounded versioned workflow and export supported exact geometry.
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
    let tolerance = Tolerance::new(doc.tolerance.linear)?;
    let step = if matches!(
        doc.operations.first(),
        Some(WorkflowOperation::RoundedBox { .. } | WorkflowOperation::ArcLineExtrusion { .. })
    ) {
        export_step_bounded_analytic_mm(&solid, tolerance.linear)?
    } else {
        export_step_mm(&solid, tolerance)?
    };
    serde_json::to_string(&serde_json::json!({"step":step,"units":"mm","schema":"AP214","faces":solid.shell.faces.len()})).map_err(|_|Error::InvalidInput("STEP export report serialization failed"))
}
