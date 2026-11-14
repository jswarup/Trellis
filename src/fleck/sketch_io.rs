// sketch_io.rs ------------------------------------------------------------------------------------------------------
//! Native `.tsketch` text format serializer/parser and SVG vector export for `SketchDocument`.

use crate::fenst::sketch::{
    SketchDocument, SketchPlane, SketchPoint, SketchPrimitive,
};
use crate::silo::IArr;

//-------------------------------------------------------------------------------------------------
// Native `.tsketch` Serialization and Parsing

pub struct SketchIo;

impl SketchIo
{
    /// Serializes a `SketchDocument` to a deterministic, human-readable `.tsketch` string.
    pub fn SerializeSketch(doc: &SketchDocument) -> String
    {
        let mut out = String::with_capacity(1024);
        out.push_str("TSKETCH 1.0\n");
        out.push_str(&format!("UNITS {}\n", doc.Units()));

        let plane = doc.Plane();
        out.push_str(&format!(
            "PLANE {:.6} {:.6} {:.6}  {:.6} {:.6} {:.6}  {:.6} {:.6} {:.6}  {:.6} {:.6} {:.6}\n",
            plane._Origin[0], plane._Origin[1], plane._Origin[2],
            plane._U[0], plane._U[1], plane._U[2],
            plane._V[0], plane._V[1], plane._V[2],
            plane._Normal[0], plane._Normal[1], plane._Normal[2],
        ));

        // Layers
        out.push_str(&format!("LAYERS {}\n", doc.LayerCount()));
        let layers = doc.Layers();
        layers.Traverse(|layer| {
            let style = layer.Style();
            out.push_str(&format!(
                "  LAYER {} \"{}\" {:.3} {:.3} {:.3} {:.3}  {:.2} {}  {} {} {}\n",
                layer.Id(),
                layer.Name(),
                style._Color[0], style._Color[1], style._Color[2], style._Color[3],
                style._LineWidth, style._LinePattern,
                layer.IsVisible(), layer.IsLocked(), layer.IsConstruction(),
            ));
        });

        // Entities
        out.push_str(&format!("ENTITIES {}\n", doc.EntityCount()));
        let arena = doc.PointArena();
        let entities = doc.Entities();
        entities.Traverse(|entity| {
            out.push_str(&format!("  ENTITY {} {} ", entity.Id(), entity.LayerId()));
            match entity.Primitive() {
                SketchPrimitive::Line(l) => {
                    out.push_str(&format!("Line ({:.6},{:.6}) ({:.6},{:.6})\n", l._Start._X, l._Start._Y, l._End._X, l._End._Y));
                }
                SketchPrimitive::Circle(c) => {
                    out.push_str(&format!("Circle ({:.6},{:.6}) {:.6}\n", c._Center._X, c._Center._Y, c._Radius));
                }
                SketchPrimitive::Arc(a) => {
                    out.push_str(&format!(
                        "Arc ({:.6},{:.6}) {:.6} {:.6} {:.6}\n",
                        a._Center._X, a._Center._Y, a._Radius, a._StartAngle, a._SweepAngle,
                    ));
                }
                SketchPrimitive::Ellipse(e) => {
                    out.push_str(&format!(
                        "Ellipse ({:.6},{:.6}) {:.6} {:.6} {:.6}\n",
                        e._Center._X, e._Center._Y, e._SemiMajor, e._SemiMinor, e._Rotation,
                    ));
                }
                SketchPrimitive::Polyline(p) => {
                    out.push_str(&format!("Polyline {} {} [", p._IsClosed, p._PointCount));
                    let mut k = 0u32;
                    while k < p._PointCount {
                        if let Some(pt) = arena.Get(p._PointOffset + k) {
                            if k > 0 {
                                out.push(' ');
                            }
                            out.push_str(&format!("({:.6},{:.6})", pt._X, pt._Y));
                        }
                        k += 1;
                    }
                    out.push_str("]\n");
                }
            }
        });

        out.push_str("END\n");
        out
    }

    /// Parses a `.tsketch` formatted string into a `SketchDocument`.
    pub fn ParseSketch(input: &str) -> Result<SketchDocument, String>
    {
        let mut lines = input.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
        let header = lines.next().ok_or_else(|| "Empty sketch file".to_string())?;
        if !header.starts_with("TSKETCH") {
            return Err(format!("Invalid file header: {}", header));
        }

        let mut doc = SketchDocument::New();

        for line in lines {
            if line == "END" {
                break;
            }
            if let Some(stripped) = line.strip_prefix("UNITS ") {
                let u = stripped.trim();
                doc.SetUnits(u);
            } else if let Some(stripped) = line.strip_prefix("PLANE ") {
                let parts: Vec<&str> = stripped.split_whitespace().collect();
                if parts.len() == 12 {
                    let mut vals = [0.0f64; 12];
                    let mut ok = true;
                    for (i, p) in parts.iter().enumerate() {
                        if let Ok(v) = p.parse::<f64>() {
                            vals[i] = v;
                        } else {
                            ok = false;
                        }
                    }
                    if ok {
                        let plane = SketchPlane::New(
                            [vals[0], vals[1], vals[2]],
                            [vals[3], vals[4], vals[5]],
                            [vals[6], vals[7], vals[8]],
                            [vals[9], vals[10], vals[11]],
                        );
                        doc.SetPlane(plane);
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("ENTITY ") {
                let rest = stripped.trim();
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if tokens.len() >= 3 {
                    let kind = tokens[2];
                    if kind == "Line" && tokens.len() >= 5 {
                        let p1 = Self::ParsePoint(tokens[3])?;
                        let p2 = Self::ParsePoint(tokens[4])?;
                        doc.AddLine(p1, p2)?;
                    } else if kind == "Circle" && tokens.len() >= 5 {
                        let center = Self::ParsePoint(tokens[3])?;
                        let radius = tokens[4].parse::<f64>().map_err(|e| e.to_string())?;
                        doc.AddCircle(center, radius)?;
                    } else if kind == "Arc" && tokens.len() >= 7 {
                        let center = Self::ParsePoint(tokens[3])?;
                        let radius = tokens[4].parse::<f64>().map_err(|e| e.to_string())?;
                        let start = tokens[5].parse::<f64>().map_err(|e| e.to_string())?;
                        let sweep = tokens[6].parse::<f64>().map_err(|e| e.to_string())?;
                        doc.AddArc(center, radius, start, sweep)?;
                    } else if kind == "Ellipse" && tokens.len() >= 7 {
                        let center = Self::ParsePoint(tokens[3])?;
                        let a = tokens[4].parse::<f64>().map_err(|e| e.to_string())?;
                        let b = tokens[5].parse::<f64>().map_err(|e| e.to_string())?;
                        let rot = tokens[6].parse::<f64>().map_err(|e| e.to_string())?;
                        doc.AddEllipse(center, a, b, rot)?;
                    }
                }
            }
        }

        Ok(doc)
    }

    fn ParsePoint(s: &str) -> Result<SketchPoint, String>
    {
        let clean = s.trim_matches(|c| c == '(' || c == ')' || c == ',');
        let parts: Vec<&str> = clean.split(',').map(|p| p.trim()).collect();
        if parts.len() != 2 {
            return Err(format!("Malformed point tuple: {}", s));
        }
        let x = parts[0].parse::<f64>().map_err(|e| e.to_string())?;
        let y = parts[1].parse::<f64>().map_err(|e| e.to_string())?;
        Ok(SketchPoint::New(x, y))
    }

    /// Exports the document geometry to an SVG document string.
    pub fn ExportSvg(doc: &SketchDocument) -> String
    {
        let bounds = doc.ComputeBounds();
        let margin = 10.0;
        let x = bounds._Min._X - margin;
        let y = bounds._Min._Y - margin;
        let w = (bounds.Width() + 2.0 * margin).max(10.0);
        let h = (bounds.Height() + 2.0 * margin).max(10.0);

        let mut svg = String::with_capacity(2048);
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\">\n",
            x, y, w, h
        ));
        svg.push_str("  <g stroke=\"black\" stroke-width=\"1.5\" fill=\"none\">\n");

        let arena = doc.PointArena();
        doc.Entities().Traverse(|ent| {
            match ent.Primitive() {
                SketchPrimitive::Line(l) => {
                    svg.push_str(&format!(
                        "    <line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" />\n",
                        l._Start._X, l._Start._Y, l._End._X, l._End._Y
                    ));
                }
                SketchPrimitive::Circle(c) => {
                    svg.push_str(&format!(
                        "    <circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" />\n",
                        c._Center._X, c._Center._Y, c._Radius
                    ));
                }
                SketchPrimitive::Arc(a) => {
                    let p1 = a.StartPoint();
                    let p2 = a.EndPoint();
                    let large_arc = if a._SweepAngle.abs() > std::f64::consts::PI { 1 } else { 0 };
                    let sweep_flag = if a._SweepAngle > 0.0 { 1 } else { 0 };
                    svg.push_str(&format!(
                        "    <path d=\"M {:.3} {:.3} A {:.3} {:.3} 0 {} {} {:.3} {:.3}\" />\n",
                        p1._X, p1._Y, a._Radius, a._Radius, large_arc, sweep_flag, p2._X, p2._Y
                    ));
                }
                SketchPrimitive::Ellipse(e) => {
                    let deg = e._Rotation.to_degrees();
                    svg.push_str(&format!(
                        "    <ellipse cx=\"{:.3}\" cy=\"{:.3}\" rx=\"{:.3}\" ry=\"{:.3}\" transform=\"rotate({:.2} {:.3} {:.3})\" />\n",
                        e._Center._X, e._Center._Y, e._SemiMajor, e._SemiMinor, deg, e._Center._X, e._Center._Y
                    ));
                }
                SketchPrimitive::Polyline(p) => {
                    let tag = if p._IsClosed { "polygon" } else { "polyline" };
                    svg.push_str(&format!("    <{} points=\"", tag));
                    let mut k = 0u32;
                    while k < p._PointCount {
                        if let Some(pt) = arena.Get(p._PointOffset + k) {
                            if k > 0 {
                                svg.push(' ');
                            }
                            svg.push_str(&format!("{:.3},{:.3}", pt._X, pt._Y));
                        }
                        k += 1;
                    }
                    svg.push_str("\" />\n");
                }
            }
        });

        svg.push_str("  </g>\n</svg>\n");
        svg
    }
}

//-------------------------------------------------------------------------------------------------
