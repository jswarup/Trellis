// sketch_flux.rs ---------------------------------------------------------------------------------
//! Exact analytic sketch schema mapping for native Flux binary and text files.

use crate::fenst::sketch::*;
use crate::flux::{FluxFile, FluxValue as V};
use crate::silo::{Buff, Stash};

pub struct SketchFlux;

impl SketchFlux
{
    fn Fields(value: &V, names: &[&str]) -> Result<(), String>
    {
        let V::Object(fields) = value else {
            return Err("Expected sketch record".into());
        };
        let mut index = 0;
        while index < fields.Size() {
            if !names.contains(&fields[index].0.as_str()) {
                return Err(format!("Unsupported sketch field {}", fields[index].0));
            }
            index += 1;
        }
        return Ok(());
    }
    fn Point(point: SketchPoint) -> V
    {
        return V::Array(crate::Buff![V::F64(point._X), V::F64(point._Y)]);
    }

    fn Vector(vector: [f64; 3]) -> V
    {
        return V::Array(Buff::FromDispenser(3, |i| V::F64(vector[i as usize])));
    }

    pub fn Payload(doc: &SketchDocument) -> V
    {
        let plane = doc.Plane();
        let layers = V::Array(Buff::FromDispenser(doc.LayerCount(), |i| {
            let layer = &doc.Layers()[i];
            let style = layer.Style();
            return V::Record(crate::Buff![("Id", V::U64(u64::from(layer.Id()))),
                                          ("Name", V::String(layer.Name().into())),
                                          ("Color",
                                           V::Array(Buff::FromDispenser(4, |i| {
                                                        V::F64(f64::from(style._Color[i as usize]))
                                                    }))),
                                          ("Width", V::F64(f64::from(style._LineWidth))),
                                          ("Pattern", V::U64(u64::from(style._LinePattern))),
                                          ("Visible", V::Bool(layer.IsVisible())),
                                          ("Locked", V::Bool(layer.IsLocked())),
                                          ("Construction", V::Bool(layer.IsConstruction())),]);
        }));
        let groups = V::Array(Buff::FromDispenser(doc.GroupCount(), |i| {
            let group = &doc.Groups()[i];
            return V::Record(crate::Buff![("Id", V::U64(u64::from(group.Id()))),
                                          ("Name", V::String(group.Name().into())),
                                          ("Members",
                                           V::Array(Buff::FromDispenser(group.MemberIds()
                                                                             .Size(),
                                                                        |j| {
                                                                            V::U64(u64::from(group.MemberIds()[j]))
                                                                        }))),]);
        }));
        let entities = V::Array(Buff::FromDispenser(doc.EntityCount(), |i| {
            let entity = &doc.Entities()[i];
            let primitive = match entity.Primitive() {
                SketchPrimitive::Line(line) => {
                    V::Record(crate::Buff![("Start", Self::Point(line._Start)),
                                           ("End", Self::Point(line._End))])
                }
                SketchPrimitive::Circle(circle) => {
                    V::Record(crate::Buff![("Center", Self::Point(circle._Center)),
                                           ("Radius", V::F64(circle._Radius))])
                }
                SketchPrimitive::Arc(arc) => {
                    V::Record(crate::Buff![("Center", Self::Point(arc._Center)),
                                           ("Radius", V::F64(arc._Radius)),
                                           ("StartAngle", V::F64(arc._StartAngle)),
                                           ("Sweep", V::F64(arc._SweepAngle)),])
                }
                SketchPrimitive::Ellipse(ellipse) => {
                    V::Record(crate::Buff![("Center", Self::Point(ellipse._Center)),
                                           ("A", V::F64(ellipse._SemiMajor)),
                                           ("B", V::F64(ellipse._SemiMinor)),
                                           ("Rotation", V::F64(ellipse._Rotation)),
                                           ("StartAngle", V::F64(ellipse._StartAngle)),
                                           ("Sweep", V::F64(ellipse._SweepAngle)),])
                }
                SketchPrimitive::Polyline(poly) => {
                    V::Record(crate::Buff![("Closed", V::Bool(poly._IsClosed)),
                                           ("Points",
                                            V::Array(Buff::FromDispenser(poly._PointCount,
                                                                         |i| {
                                                                             Self::Point(doc.PointArena()[poly._PointOffset + i])
                                                                         }))),])
                }
            };
            return V::Record(crate::Buff![("Type",
                                           V::String(entity.Primitive().KindName().into())),
                                          ("Id", V::U64(u64::from(entity.Id()))),
                                          ("Layer", V::U64(u64::from(entity.LayerId()))),
                                          ("Group",
                                           if entity.GroupId() == u32::MAX {
                                               V::Null
                                           } else {
                                               V::U64(u64::from(entity.GroupId()))
                                           }),
                                          ("Construction", V::Bool(entity.IsConstruction())),
                                          ("Geometry", primitive),]);
        }));
        let dimensions = V::Array(Buff::FromDispenser(doc.DimensionCount(), |i| {
            let dim = &doc.Dimensions()[i];
            let kind_str = match dim._Kind {
                crate::fenst::sketch_dimension::DimensionKind::LinearAligned => "LinearAligned",
                crate::fenst::sketch_dimension::DimensionKind::LinearHorizontal => "LinearHorizontal",
                crate::fenst::sketch_dimension::DimensionKind::LinearVertical => "LinearVertical",
                crate::fenst::sketch_dimension::DimensionKind::Radial => "Radial",
                crate::fenst::sketch_dimension::DimensionKind::Diametral => "Diametral",
                crate::fenst::sketch_dimension::DimensionKind::Angular => "Angular",
            };
            V::Record(crate::Buff![
                ("Id", V::U64(u64::from(dim._Id))),
                ("Kind", V::String(kind_str.into())),
                ("P1", Self::Point(dim._P1)),
                ("P2", Self::Point(dim._P2)),
                ("Offset", V::F64(dim._Offset)),
                ("Target", if dim._TargetEntityId == u32::MAX { V::Null } else { V::U64(u64::from(dim._TargetEntityId)) }),
            ])
        }));
        return V::Record(crate::Buff![("Units", V::String(doc.Units().into())),
                                      ("Plane",
                                       V::Record(crate::Buff![("Origin",
                                                               Self::Vector(plane._Origin)),
                                                              ("U", Self::Vector(plane._U)),
                                                              ("V", Self::Vector(plane._V)),
                                                              ("Normal",
                                                               Self::Vector(plane._Normal)),])),
                                      ("Layers", layers),
                                      ("Groups", groups),
                                      ("Entities", entities),
                                      ("Dimensions", dimensions),]);
    }

    pub fn File(doc: &SketchDocument, previous: Option<&V>) -> Result<FluxFile, String>
    {
        if doc.EntityCount() > 50_000 {
            return Err("Sketch document exceeds maximum entity capacity (50,000 entities)".into());
        }
        let payload = Self::Payload(doc);
        let metadata = match previous {
            Some(V::Object(fields)) => {
                V::Object(Buff::FromDispenser(fields.Size(), |i| {
                    let (name, value) = &fields[i];
                    (name.clone(),
                     if name == "DocumentRevision" {
                         V::U64(doc.Revision())
                     } else {
                         value.clone()
                     })
                }))
            }
            _ => {
                V::Record(crate::Buff![
                    ("Schema", V::String("trellis.sketch".into())),
                    ("SchemaVersion", V::U64(1)),
                    ("DocumentId", V::String(FluxFile::NewId()?)),
                    ("DocumentRevision", V::U64(doc.Revision())),
                    ("CreatedUtc", V::Null),
                    ("ModifiedUtc", V::Null),
                    (
                        "Producer",
                        V::Record(crate::Buff![
                            ("Name", V::String("Trellis".into())),
                            ("Version", V::String(env!("CARGO_PKG_VERSION").into())),
                        ])
                    ),
                ])
            }
        };
        FluxFile::FromParts(metadata, payload)
    }

    fn Id(value: &V) -> Result<u32, String>
    {
        let id = u32::try_from(value.UInt()?).map_err(|_| "Sketch ID exceeds u32")?;
        if id >= u32::MAX - 1 {
            return Err("Reserved or exhausted sketch ID".into());
        }
        return Ok(id);
    }

    fn ReadPoint(value: &V) -> Result<SketchPoint, String>
    {
        let points = value.Items()?;
        if points.Size() != 2 {
            return Err("A sketch point must have two coordinates".into());
        }
        let point = SketchPoint::New(points[0].Real()?, points[1].Real()?);
        if !point.IsFinite() {
            return Err("Sketch coordinates must be finite".into());
        }
        return Ok(point);
    }

    fn ReadVector(value: &V) -> Result<[f64; 3], String>
    {
        let values = value.Items()?;
        if values.Size() != 3 {
            return Err("A workplane vector needs three coordinates".into());
        }
        return Ok([values[0].Real()?, values[1].Real()?, values[2].Real()?]);
    }

    pub fn Document(file: &FluxFile) -> Result<SketchDocument, String>
    {
        if file.Metadata().Get("Schema")?.Text()? != "trellis.sketch"
           || file.Metadata().Get("SchemaVersion")?.UInt()? != 1
        {
            return Err("This file is not a supported trellis.sketch document (version 1)".into());
        }
        let root = file.Payload();
        Self::Fields(root, &["Units", "Plane", "Layers", "Groups", "Entities", "Dimensions"])?;
        let plane = root.Get("Plane")?;
        Self::Fields(plane, &["Origin", "U", "V", "Normal"])?;
        let plane = SketchPlane::New(Self::ReadVector(plane.Get("Origin")?)?,
                                     Self::ReadVector(plane.Get("U")?)?,
                                     Self::ReadVector(plane.Get("V")?)?,
                                     Self::ReadVector(plane.Get("Normal")?)?);
        let mut layers = Stash::New();
        let values = root.Get("Layers")?.Items()?;
        let mut i = 0;
        while i < values.Size() {
            let v = &values[i];
            Self::Fields(v,
                         &["Id",
                           "Name",
                           "Color",
                           "Width",
                           "Pattern",
                           "Visible",
                           "Locked",
                           "Construction"])?;
            let color = v.Get("Color")?.Items()?;
            if color.Size() != 4 {
                return Err("Layer color needs four components".into());
            }
            let mut rgba = [0f32; 4];
            let mut j = 0;
            while j < 4 {
                let component = color[j].Real()?;
                if !(0.0..=1.0).contains(&component) {
                    return Err("Invalid layer color".into());
                }
                rgba[j as usize] = component as f32;
                j += 1;
            }
            let width = v.Get("Width")?.Real()?;
            if !width.is_finite() || width <= 0.0 || width > 100.0 {
                return Err("Invalid layer stroke width".into());
            }
            let mut layer = SketchLayer::New(Self::Id(v.Get("Id")?)?, v.Get("Name")?.Text()?);
            layer.SetStyle(SketchStyle { _Color: rgba, _LineWidth: width as f32, _LinePattern: u8::try_from(v.Get("Pattern")?.UInt()?).map_err(|_| "Invalid line pattern")? });
            layer.SetVisible(v.Get("Visible")?.Boolean()?);
            layer.SetLocked(v.Get("Locked")?.Boolean()?);
            layer.SetConstruction(v.Get("Construction")?.Boolean()?);
            layers.Push(layer);
            i += 1;
        }
        let mut doc = SketchDocument::New();
        let values = root.Get("Entities")?.Items()?;
        i = 0;
        while i < values.Size() {
            let v = &values[i];
            Self::Fields(v,
                         &["Type", "Id", "Layer", "Group", "Construction", "Geometry"])?;
            let id = Self::Id(v.Get("Id")?)?;
            if doc.FindEntity(id).is_some() {
                return Err(format!("Duplicate sketch entity ID {id}"));
            }
            let geo = v.Get("Geometry")?;
            let generated = match v.Get("Type")?.Text()? {
                "Line" => {
                    Self::Fields(geo, &["Start", "End"])?;
                    doc.AddLine(Self::ReadPoint(geo.Get("Start")?)?,
                                Self::ReadPoint(geo.Get("End")?)?)?
                }
                "Circle" => {
                    Self::Fields(geo, &["Center", "Radius"])?;
                    doc.AddCircle(Self::ReadPoint(geo.Get("Center")?)?,
                                  geo.Get("Radius")?.Real()?)?
                }
                "Arc" => {
                    Self::Fields(geo, &["Center", "Radius", "StartAngle", "Sweep"])?;
                    doc.AddArc(Self::ReadPoint(geo.Get("Center")?)?,
                               geo.Get("Radius")?.Real()?,
                               geo.Get("StartAngle")?.Real()?,
                               geo.Get("Sweep")?.Real()?)?
                }
                "Ellipse" | "EllipticalArc" => {
                    Self::Fields(geo,
                                 &["Center", "A", "B", "Rotation", "StartAngle", "Sweep"])?;
                    let generated = doc.AddEllipse(Self::ReadPoint(geo.Get("Center")?)?,
                                                   geo.Get("A")?.Real()?,
                                                   geo.Get("B")?.Real()?,
                                                   geo.Get("Rotation")?.Real()?)?;
                    let start = geo.Get("StartAngle")?.Real()?;
                    let sweep = geo.Get("Sweep")?.Real()?;
                    if !start.is_finite()
                       || !sweep.is_finite()
                       || sweep.abs() <= 1e-9
                       || sweep.abs() > std::f64::consts::TAU
                    {
                        return Err("Invalid elliptical arc angles".into());
                    }
                    if let Some(entity) = doc.FindEntityMut(generated)
                       && let SketchPrimitive::Ellipse(ellipse) = entity.PrimitiveMut()
                    {
                        ellipse._StartAngle = start;
                        ellipse._SweepAngle = sweep;
                    }
                    generated
                }
                "Polyline" => {
                    Self::Fields(geo, &["Closed", "Points"])?;
                    let points = geo.Get("Points")?.Items()?;
                    let mut buffer = Stash::New();
                    let mut j = 0;
                    while j < points.Size() {
                        buffer.Push(Self::ReadPoint(&points[j])?);
                        j += 1;
                    }
                    doc.AddPolyline(buffer.Arr(), geo.Get("Closed")?.Boolean()?)?
                }
                kind => return Err(format!("Unknown sketch primitive {kind}")),
            };
            let temporary = doc.RemoveEntity(generated)?;
            let group = match v.Get("Group")? {
                V::Null => u32::MAX,
                v => Self::Id(v)?,
            };
            doc.InsertEntity(SketchEntity::New(id, *temporary.Primitive()).WithLayer(Self::Id(v.Get("Layer")?)?).WithGroup(group).WithConstruction(v.Get("Construction")?.Boolean()?));
            i += 1;
        }
        let mut groups = Stash::New();
        let values = root.Get("Groups")?.Items()?;
        i = 0;
        while i < values.Size() {
            let v = &values[i];
            Self::Fields(v, &["Id", "Name", "Members"])?;
            let members = v.Get("Members")?.Items()?;
            let mut ids = Stash::New();
            let mut j = 0;
            while j < members.Size() {
                ids.Push(Self::Id(&members[j])?);
                j += 1;
            }
            groups.Push(SketchGroup::New(Self::Id(v.Get("Id")?)?,
                                         v.Get("Name")?.Text()?,
                                         ids.IntoBuff()));
            i += 1;
        }
        if let Ok(dim_val) = root.Get("Dimensions") {
            let dim_items = dim_val.Items()
                .map_err(|_| String::from("Invalid Dimensions collection payload"))?;
            let mut d = 0;
            let mut seen_ids = std::collections::BTreeSet::new();
            while d < dim_items.Size() {
                let dv = &dim_items[d];
                let id = Self::Id(dv.Get("Id")?)?;
                if !seen_ids.insert(id) {
                    return Err(format!("Duplicate dimension ID: {}", id));
                }
                let kind_str = dv.Get("Kind")?.Text()?;
                let kind = match kind_str {
                    "LinearAligned" => crate::fenst::sketch_dimension::DimensionKind::LinearAligned,
                    "LinearHorizontal" => crate::fenst::sketch_dimension::DimensionKind::LinearHorizontal,
                    "LinearVertical" => crate::fenst::sketch_dimension::DimensionKind::LinearVertical,
                    "Radial" => crate::fenst::sketch_dimension::DimensionKind::Radial,
                    "Diametral" => crate::fenst::sketch_dimension::DimensionKind::Diametral,
                    "Angular" => crate::fenst::sketch_dimension::DimensionKind::Angular,
                    other => return Err(format!("Unknown dimension kind: {}", other)),
                };
                let p1 = Self::ReadPoint(dv.Get("P1")?)?;
                let p2 = Self::ReadPoint(dv.Get("P2")?)?;
                if !p1._X.is_finite() || !p1._Y.is_finite() || !p2._X.is_finite() || !p2._Y.is_finite() {
                    return Err("Nonfinite dimension coordinates".into());
                }
                let offset = dv.Get("Offset")?.Real()?;
                if !offset.is_finite() {
                    return Err("Nonfinite dimension offset".into());
                }
                let target = match dv.Get("Target")? {
                    V::Null => u32::MAX,
                    v => Self::Id(v)?,
                };
                if target != u32::MAX && doc.FindEntity(target).is_none() {
                    return Err(format!("Dangling dimension target entity ID: {}", target));
                }
                let dim = crate::fenst::sketch_dimension::SketchDimension::New(
                    id,
                    kind,
                    p1,
                    p2,
                    offset,
                    target,
                );
                doc.InsertDimension(dim);
                d += 1;
            }
        }
        doc.RestoreSettings(root.Get("Units")?.Text()?,
                            plane,
                            layers.Arr(),
                            groups.Arr(),
                            file.Metadata().Get("DocumentRevision")?.UInt()?)?;
        return Ok(doc);
    }
}

//-------------------------------------------------------------------------------------------------
