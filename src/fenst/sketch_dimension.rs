// sketch_dimension.rs ----------------------------------------------------------------------------------------------
//! Engineering dimensions and annotations compliant with ISO 129 / ANSI Y14.5 drafting standards.

use crate::fenst::sketch::SketchPoint;

//-------------------------------------------------------------------------------------------------

/// Standard engineering dimension classification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DimensionKind
{
    /// Aligned linear dimension measuring Euclidean distance along the segment direction.
    LinearAligned,
    /// Horizontal linear dimension measuring delta X.
    LinearHorizontal,
    /// Vertical linear dimension measuring delta Y.
    LinearVertical,
    /// Radial dimension callout for circles or circular arcs (`R` prefix).
    Radial,
    /// Diametral dimension callout across circle/arc center (`⌀` prefix).
    Diametral,
    /// Angular dimension measuring acute or obtuse angle between two vectors (`°`).
    Angular,
}

impl DimensionKind
{
    pub fn DisplayName(&self) -> &'static str
    {
        match self {
            Self::LinearAligned => "Linear Aligned",
            Self::LinearHorizontal => "Horizontal",
            Self::LinearVertical => "Vertical",
            Self::Radial => "Radius",
            Self::Diametral => "Diameter",
            Self::Angular => "Angle",
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Represents an engineering dimension annotation with ISO/ANSI extension lines and arrowheads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchDimension
{
    pub _Id:             u32,
    pub _Kind:           DimensionKind,
    pub _P1:             SketchPoint,
    pub _P2:             SketchPoint,
    pub _Offset:         f64,
    pub _TextPos:        SketchPoint,
    pub _TargetEntityId: u32,
}

impl SketchDimension
{
    /// Creates a new engineering dimension annotation.
    pub fn New(
        id: u32,
        kind: DimensionKind,
        p1: SketchPoint,
        p2: SketchPoint,
        offset: f64,
        target_entity_id: u32,
    ) -> Self
    {
        let text_pos = Self::ComputeDefaultTextPos(kind, p1, p2, offset);
        Self {
            _Id: id,
            _Kind: kind,
            _P1: p1,
            _P2: p2,
            _Offset: offset,
            _TextPos: text_pos,
            _TargetEntityId: target_entity_id,
        }
    }

    /// Measures the scalar engineering value (distance in mm or angle in degrees).
    pub fn Value(&self) -> f64
    {
        match self._Kind {
            DimensionKind::LinearAligned => self._P1.DistanceTo(self._P2),
            DimensionKind::LinearHorizontal => (self._P2._X - self._P1._X).abs(),
            DimensionKind::LinearVertical => (self._P2._Y - self._P1._Y).abs(),
            DimensionKind::Radial => self._P1.DistanceTo(self._P2),
            DimensionKind::Diametral => self._P1.DistanceTo(self._P2) * 2.0,
            DimensionKind::Angular => {
                let dx1 = self._P1._X;
                let dy1 = self._P1._Y;
                let dx2 = self._P2._X;
                let dy2 = self._P2._Y;
                let dot = dx1 * dx2 + dy1 * dy2;
                let len1 = (dx1 * dx1 + dy1 * dy1).sqrt();
                let len2 = (dx2 * dx2 + dy2 * dy2).sqrt();
                if len1 < 1e-6 || len2 < 1e-6 {
                    0.0
                } else {
                    let cos_a = (dot / (len1 * len2)).clamp(-1.0, 1.0);
                    cos_a.acos().to_degrees()
                }
            }
        }
    }

    /// Formats the engineering callout text according to ISO/ANSI standards.
    pub fn FormattedText(&self, units: &str) -> String
    {
        let val = self.Value();
        match self._Kind {
            DimensionKind::LinearAligned | DimensionKind::LinearHorizontal | DimensionKind::LinearVertical => {
                if units.is_empty() {
                    format!("{:.2}", val)
                } else {
                    format!("{:.2} {}", val, units)
                }
            }
            DimensionKind::Radial => {
                if units.is_empty() {
                    format!("R{:.2}", val)
                } else {
                    format!("R{:.2} {}", val, units)
                }
            }
            DimensionKind::Diametral => {
                if units.is_empty() {
                    format!("⌀{:.2}", val)
                } else {
                    format!("⌀{:.2} {}", val, units)
                }
            }
            DimensionKind::Angular => format!("{:.1}°", val),
        }
    }

    /// Computes the dimension line endpoints (where arrowheads touch).
    pub fn DimensionLine(&self) -> (SketchPoint, SketchPoint)
    {
        match self._Kind {
            DimensionKind::LinearAligned => {
                let dx = self._P2._X - self._P1._X;
                let dy = self._P2._Y - self._P1._Y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-6 {
                    return (self._P1, self._P2);
                }
                // Normal vector perpendicular to P1->P2
                let nx = -dy / len;
                let ny = dx / len;
                let d1 = SketchPoint::New(self._P1._X + nx * self._Offset, self._P1._Y + ny * self._Offset);
                let d2 = SketchPoint::New(self._P2._X + nx * self._Offset, self._P2._Y + ny * self._Offset);
                (d1, d2)
            }
            DimensionKind::LinearHorizontal => {
                let y = self._P1._Y.max(self._P2._Y) + self._Offset;
                (SketchPoint::New(self._P1._X, y), SketchPoint::New(self._P2._X, y))
            }
            DimensionKind::LinearVertical => {
                let x = self._P1._X.max(self._P2._X) + self._Offset;
                (SketchPoint::New(x, self._P1._Y), SketchPoint::New(x, self._P2._Y))
            }
            DimensionKind::Radial | DimensionKind::Diametral => {
                (self._P1, self._P2)
            }
            DimensionKind::Angular => {
                (self._P1, self._P2)
            }
        }
    }

    /// Computes ISO extension lines: `((ext1_start, ext1_end), (ext2_start, ext2_end))`.
    pub fn ExtensionLines(&self) -> ((SketchPoint, SketchPoint), (SketchPoint, SketchPoint))
    {
        let (d1, d2) = self.DimensionLine();
        const OVERSHOOT: f64 = 3.0; // 3mm standard extension line overshoot

        match self._Kind {
            DimensionKind::LinearAligned => {
                let dx = self._P2._X - self._P1._X;
                let dy = self._P2._Y - self._P1._Y;
                let len = (dx * dx + dy * dy).sqrt();
                let (nx, ny) = if len > 1e-6 { (-dy / len, dx / len) } else { (0.0, 1.0) };
                let sign = if self._Offset >= 0.0 { 1.0 } else { -1.0 };
                let ext1_end = SketchPoint::New(d1._X + nx * sign * OVERSHOOT, d1._Y + ny * sign * OVERSHOOT);
                let ext2_end = SketchPoint::New(d2._X + nx * sign * OVERSHOOT, d2._Y + ny * sign * OVERSHOOT);
                ((self._P1, ext1_end), (self._P2, ext2_end))
            }
            DimensionKind::LinearHorizontal => {
                let sign = if self._Offset >= 0.0 { 1.0 } else { -1.0 };
                let y_end = d1._Y + sign * OVERSHOOT;
                (
                    (self._P1, SketchPoint::New(self._P1._X, y_end)),
                    (self._P2, SketchPoint::New(self._P2._X, y_end)),
                )
            }
            DimensionKind::LinearVertical => {
                let sign = if self._Offset >= 0.0 { 1.0 } else { -1.0 };
                let x_end = d1._X + sign * OVERSHOOT;
                (
                    (self._P1, SketchPoint::New(x_end, self._P1._Y)),
                    (self._P2, SketchPoint::New(x_end, self._P2._Y)),
                )
            }
            _ => ((self._P1, self._P1), (self._P2, self._P2)),
        }
    }

    /// Computes ISO 3:1 aspect ratio closed arrowheads at both ends of the dimension line.
    pub fn Arrowheads(&self, arrow_len: f64) -> ([SketchPoint; 3], [SketchPoint; 3])
    {
        let (d1, d2) = self.DimensionLine();
        let dx = d2._X - d1._X;
        let dy = d2._Y - d1._Y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return ([d1, d1, d1], [d2, d2, d2]);
        }

        let ux = dx / len;
        let uy = dy / len;
        let nx = -uy;
        let ny = ux;
        let half_width = arrow_len * 0.35; // standard 3:1 taper

        // Arrow 1 at d1 pointing toward d2 (or outwards)
        let a1_base = SketchPoint::New(d1._X + ux * arrow_len, d1._Y + uy * arrow_len);
        let a1_left = SketchPoint::New(a1_base._X + nx * half_width, a1_base._Y + ny * half_width);
        let a1_right = SketchPoint::New(a1_base._X - nx * half_width, a1_base._Y - ny * half_width);

        // Arrow 2 at d2 pointing toward d1
        let a2_base = SketchPoint::New(d2._X - ux * arrow_len, d2._Y - uy * arrow_len);
        let a2_left = SketchPoint::New(a2_base._X + nx * half_width, a2_base._Y + ny * half_width);
        let a2_right = SketchPoint::New(a2_base._X - nx * half_width, a2_base._Y - ny * half_width);

        ([d1, a1_left, a1_right], [d2, a2_left, a2_right])
    }

    pub fn ComputeDefaultTextPos(
        kind: DimensionKind,
        p1: SketchPoint,
        p2: SketchPoint,
        offset: f64,
    ) -> SketchPoint
    {
        match kind {
            DimensionKind::LinearAligned => {
                let dx = p2._X - p1._X;
                let dy = p2._Y - p1._Y;
                let len = (dx * dx + dy * dy).sqrt();
                let (nx, ny) = if len > 1e-6 { (-dy / len, dx / len) } else { (0.0, 1.0) };
                let mid_x = (p1._X + p2._X) * 0.5 + nx * offset;
                let mid_y = (p1._Y + p2._Y) * 0.5 + ny * offset;
                SketchPoint::New(mid_x, mid_y)
            }
            DimensionKind::LinearHorizontal => {
                let y = p1._Y.max(p2._Y) + offset;
                SketchPoint::New((p1._X + p2._X) * 0.5, y)
            }
            DimensionKind::LinearVertical => {
                let x = p1._X.max(p2._X) + offset;
                SketchPoint::New(x, (p1._Y + p2._Y) * 0.5)
            }
            DimensionKind::Radial | DimensionKind::Diametral => {
                SketchPoint::New((p1._X + p2._X) * 0.5, (p1._Y + p2._Y) * 0.5 + 4.0)
            }
            DimensionKind::Angular => {
                SketchPoint::New((p1._X + p2._X) * 0.5, (p1._Y + p2._Y) * 0.5)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
