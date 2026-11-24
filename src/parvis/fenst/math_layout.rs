// math_layout.rs ----------------------------------------------------------------------------------
//! Backend-neutral mathematical typesetter, baseline-aware box layout, and hit-testing engine.

use crate::fenst::cask::Rect;
use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fresco::{ExprReadError, ExprView, LogicOp};
use crate::silo::{Arr, Buff, Stash, USeg};

//-------------------------------------------------------------------------------------------------

/// Semantic colors resolved by the active rendering palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathColor
{
    Text,
    Accent,
    Muted,
    Operator,
    Selection,
}

//-------------------------------------------------------------------------------------------------

/// Text metrics returned by a font measurement adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MathTextMetrics
{
    pub width:                   f32,
    /// Glyph ink relative to the text origin and baseline; independent of advance.
    pub inkBounds:               Rect,
    pub ascent:                  f32,
    pub descent:                 f32,
    pub paragraphBaselineOffset: f32,
}

/// Abstract font metrics provider for measuring math text.
pub trait IMathFontMetrics
{
    fn MeasureAtom(&self, text: &str, fontSize: f32, isBold: bool, isItalic: bool)
                   -> MathTextMetrics;
}

//-------------------------------------------------------------------------------------------------

/// Deterministic mock font metrics provider for headless tests.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockFontMetrics;

impl IMathFontMetrics for MockFontMetrics
{
    fn MeasureAtom(&self, text: &str, fontSize: f32, _is_bold: bool, is_italic: bool)
                   -> MathTextMetrics
    {
        let charCount = text.chars().count() as f32;
        let width = (charCount * fontSize * 0.55).max(fontSize * 0.3);
        let inkWidth = if is_italic { width * 1.3 } else { width };
        let ascent = fontSize * 0.8;
        let descent = fontSize * 0.2;
        MathTextMetrics { width,
                          inkBounds: Rect::New(0.0, -ascent, inkWidth, ascent + descent),
                          ascent,
                          descent,
                          paragraphBaselineOffset: ascent }
    }
}

//-------------------------------------------------------------------------------------------------

/// Delimiter bracket shape kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelimiterKind
{
    ParenthesisLeft,
    ParenthesisRight,
    BracketLeft,
    BracketRight,
    BraceLeft,
    BraceRight,
}

//-------------------------------------------------------------------------------------------------

/// Backend-neutral mathematical draw commands.
#[derive(Clone, Debug, PartialEq)]
pub enum MathDrawCommand
{
    Text
    {
        text:                    String,
        x:                       f32,
        yBaseline:               f32,
        paragraphBaselineOffset: f32,
        fontSize:                f32,
        isBold:                  bool,
        isItalic:                bool,
        color:                   MathColor,
    },
    Rule
    {
        bounds: Rect, color: MathColor
    },
    Delimiter
    {
        kind:      DelimiterKind,
        bounds:    Rect,
        thickness: f32,
        color:     MathColor,
    },
}

//-------------------------------------------------------------------------------------------------

/// Coordinates retain semantic rank independently of the displayed grid orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathCoordinates
{
    _Indices: [u32; 2],
    _Rank:    u32,
}

impl MathCoordinates
{
    pub fn Vector(index: u32) -> Self
    {
        return Self { _Indices: [index, 0],
                      _Rank:    1, };
    }
    pub fn Matrix(row: u32, col: u32) -> Self
    {
        return Self { _Indices: [row, col],
                      _Rank:    2, };
    }
    pub fn Rank(&self) -> u32 { return self._Rank; }
    pub fn Indices(&self) -> Arr<'_, u32>
    {
        return Arr::from(&self._Indices[..self._Rank as usize]);
    }
}

impl core::fmt::Display for MathCoordinates
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        if self._Rank == 1 {
            return write!(f, "[{}]", self._Indices[0]);
        }
        return write!(f, "[{}, {}]", self._Indices[0], self._Indices[1]);
    }
}

//-------------------------------------------------------------------------------------------------

/// Semantic occurrence mapping a rendered box to an expression node or tensor coordinate.
#[derive(Clone, Debug, PartialEq)]
pub struct MathOccurrence
{
    pub _Id:        u32,
    pub _ExprId:    u32,
    pub _Bounds:    Rect,
    pub _RootIndex: u32,
    pub _Coords:    Option<MathCoordinates>,
}

//-------------------------------------------------------------------------------------------------

/// Kinds of mathematical layout boxes.
#[derive(Clone, Debug, PartialEq)]
pub enum MathBoxKind
{
    Atom
    {
        text:                    String,
        isBold:                  bool,
        isItalic:                bool,
        color:                   MathColor,
        paragraphBaselineOffset: f32,
    },
    Row
    {
        spacing: f32
    },
    Fraction
    {
        ruleThickness: f32,
        gap:           f32,
        color:         MathColor,
    },
    Script
    {
        hasSub: bool, hasSup: bool
    },
    Delimiter
    {
        kindLeft:  DelimiterKind,
        kindRight: DelimiterKind,
        thickness: f32,
        color:     MathColor,
    },
    Grid
    {
        rows:         u32,
        cols:         u32,
        omittedRows:  u32,
        omittedCols:  u32,
        colSpacing:   f32,
        rowSpacing:   f32,
        delimBracket: bool,
        color:        MathColor,
    },
}

//-------------------------------------------------------------------------------------------------

/// A measured and positioned box in the math layout arena.
#[derive(Clone, Debug, PartialEq)]
pub struct MathBox
{
    pub _Kind:          MathBoxKind,
    pub _Width:         f32,
    pub _InkBounds:     Rect,
    pub _Ascent:        f32,
    pub _Descent:       f32,
    pub _ChildrenStart: u32,
    pub _ChildrenCount: u32,
    pub _OccurrenceId:  u32,
    pub _RelX:          f32,
    pub _RelY:          f32,
    pub _AbsBounds:     Rect,
    pub _FontSize:      f32,
}

impl MathBox
{
    pub fn Height(&self) -> f32 { self._Ascent + self._Descent }
}

//-------------------------------------------------------------------------------------------------

/// Budget limits for math layout construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathLayoutLimits
{
    pub _MaxDepth:          u32,
    pub _MaxBoxes:          u32,
    pub _MaxOccurrences:    u32,
    pub _MaxGridRows:       u32,
    pub _MaxGridCols:       u32,
    pub _MaxIdentifierLen:  u32,
    pub _MaxTotalTextBytes: u32,
}

impl Default for MathLayoutLimits
{
    fn default() -> Self
    {
        Self { _MaxDepth:          64,
               _MaxBoxes:          4096,
               _MaxOccurrences:    2048,
               _MaxGridRows:       16,
               _MaxGridCols:       16,
               _MaxIdentifierLen:  128,
               _MaxTotalTextBytes: 65536, }
    }
}

//-------------------------------------------------------------------------------------------------

/// Completed math layout containing boxes, occurrences, bounds, and draw commands.
pub struct MathLayout
{
    _Boxes:        Buff<MathBox>,
    _ChildIndices: Buff<u32>,
    _Occurrences:  Buff<MathOccurrence>,
    _Commands:     Buff<MathDrawCommand>,
    _RootBox:      u32,
    _TotalBounds:  Rect,
}

impl MathLayout
{
    #[inline]
    pub fn Bounds(&self) -> Rect { self._TotalBounds }

    #[inline]
    pub fn Commands(&self) -> &Buff<MathDrawCommand> { &self._Commands }

    #[inline]
    pub fn Occurrences(&self) -> &Buff<MathOccurrence> { &self._Occurrences }

    pub fn OccurrenceAt(&self, id: u32) -> Option<&MathOccurrence>
    {
        if id < self._Occurrences.Size() {
            Some(&self._Occurrences[id])
        } else {
            None
        }
    }

    /// Performs reverse-order hit testing to return the smallest eligible occurrence at (px, py).
    pub fn HitTest(&self, px: f32, py: f32) -> u32
    {
        let count = self._Occurrences.Size();
        if count == 0 {
            return u32::MAX;
        }

        let mut bestId = u32::MAX;
        let mut bestArea = f32::MAX;

        let mut i = 0u32;
        while i < count {
            let occ = &self._Occurrences[i];
            if occ._Bounds.Contains(px, py) {
                let area = occ._Bounds.width * occ._Bounds.height;
                if area <= bestArea {
                    bestArea = area;
                    bestId = occ._Id;
                }
            }
            i += 1;
        }

        bestId
    }

    /// Exports the validated layout draw commands into SVG string format observing output budgets.
    pub fn ExportSvg(&self, options: &MathExportOptions) -> Result<String, String>
    {
        let pad = options._Padding;
        let bounds = self._TotalBounds;
        let vx = bounds.x - pad;
        let vy = bounds.y - pad;
        let vw = (bounds.width + 2.0 * pad).max(1.0);
        let vh = (bounds.height + 2.0 * pad).max(1.0);

        let mut svg = String::with_capacity(1024);
        let header = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{:.2} {:.2} {:.2} {:.2}" width="{:.2}" height="{:.2}">"#,
                             vx, vy, vw, vh, vw, vh);
        svg.push_str(&header);
        svg.push('\n');

        if options._IncludeBackground {
            let bg = format!(r##"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#ffffff" />"##,
                             vx, vy, vw, vh);
            svg.push_str(&bg);
            svg.push('\n');
        }

        let cmdCount = self._Commands.Size();
        let mut k = 0u32;
        while k < cmdCount {
            match &self._Commands[k] {
                MathDrawCommand::Text { text,
                                        x,
                                        yBaseline,
                                        fontSize,
                                        isBold,
                                        isItalic,
                                        color,
                                        .. } => {
                    let escaped = EscapeMathXml(text);
                    let fontW = if *isBold { "bold" } else { "normal" };
                    let fontS = if *isItalic { "italic" } else { "normal" };
                    let colorHex = match color {
                        MathColor::Text => "#1a1a1a",
                        MathColor::Accent => "#0066cc",
                        MathColor::Muted => "#737373",
                        MathColor::Operator => "#990000",
                        MathColor::Selection => "#0055ff",
                    };
                    let elem = format!(r#"  <text x="{:.2}" y="{:.2}" font-size="{:.2}" font-weight="{}" font-style="{}" fill="{}" font-family="sans-serif">{}</text>"#,
                                       x, yBaseline, fontSize, fontW, fontS, colorHex, escaped);
                    if svg.len() + elem.len() > options._MaxOutputBytes as usize {
                        return Err("SVG export exceeded output byte budget".to_string());
                    }
                    svg.push_str(&elem);
                    svg.push('\n');
                }
                MathDrawCommand::Rule { bounds: rBounds,
                                        color, } => {
                    let colorHex = match color {
                        MathColor::Operator => "#990000",
                        _ => "#1a1a1a",
                    };
                    let elem = format!(r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" />"#,
                                       rBounds.x,
                                       rBounds.y,
                                       rBounds.width,
                                       rBounds.height,
                                       colorHex);
                    if svg.len() + elem.len() > options._MaxOutputBytes as usize {
                        return Err("SVG export exceeded output byte budget".to_string());
                    }
                    svg.push_str(&elem);
                    svg.push('\n');
                }
                MathDrawCommand::Delimiter { bounds: dBounds,
                                             thickness,
                                             color,
                                             .. } => {
                    let colorHex = match color {
                        MathColor::Operator => "#990000",
                        _ => "#1a1a1a",
                    };
                    let elem = format!(r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" stroke="{}" stroke-width="{:.2}" fill="none" />"#,
                                       dBounds.x,
                                       dBounds.y,
                                       dBounds.width,
                                       dBounds.height,
                                       colorHex,
                                       thickness);
                    if svg.len() + elem.len() > options._MaxOutputBytes as usize {
                        return Err("SVG export exceeded output byte budget".to_string());
                    }
                    svg.push_str(&elem);
                    svg.push('\n');
                }
            }
            k += 1;
        }

        svg.push_str("</svg>\n");
        if svg.len() > options._MaxOutputBytes as usize {
            return Err("SVG export exceeded output byte budget".to_string());
        }
        return Ok(svg);
    }

    /// Exports the validated text layout commands as a readable linear display-text string.
    pub fn ExportDisplayText(&self, maxBytes: u32) -> Result<String, String>
    {
        let mut out = String::with_capacity(256);
        let cmdCount = self._Commands.Size();
        let mut k = 0u32;
        while k < cmdCount {
            if let MathDrawCommand::Text { text, .. } = &self._Commands[k] {
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                if out.len() + text.len() > maxBytes as usize {
                    return Err("Display text export exceeded output byte budget".to_string());
                }
                out.push_str(text);
            }
            k += 1;
        }
        return Ok(out);
    }
}

//-------------------------------------------------------------------------------------------------

/// Options controlling SVG serialization of mathematical layouts.
#[derive(Clone, Copy, Debug)]
pub struct MathExportOptions
{
    pub _Padding:           f32,
    pub _MaxOutputBytes:    u32,
    pub _IncludeBackground: bool,
}

impl Default for MathExportOptions
{
    fn default() -> Self
    {
        Self {
            _Padding:           10.0,
            _MaxOutputBytes:    65536,
            _IncludeBackground: true,
        }
    }
}

fn EscapeMathXml(s: &str) -> String
{
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    return out;
}

//-------------------------------------------------------------------------------------------------

/// Internal builder constructing the arena and placing boxes.
pub struct MathLayoutBuilder<'a, M: IMathFontMetrics>
{
    _Document:       &'a FrescoDocument,
    _Metrics:        &'a M,
    _Limits:         MathLayoutLimits,
    _BaseFontSize:   f32,
    _RootIndex:      u32,
    _Boxes:          Stash<MathBox>,
    _ChildIndices:   Stash<u32>,
    _Occurrences:    Stash<MathOccurrence>,
    _Commands:       Stash<MathDrawCommand>,
    _TotalTextBytes: u32,
}

impl<'a, M: IMathFontMetrics> MathLayoutBuilder<'a, M>
{
    pub fn New(doc: &'a FrescoDocument, metrics: &'a M, rootIndex: u32, baseFontSize: f32) -> Self
    {
        Self { _Document:       doc,
               _Metrics:        metrics,
               _Limits:         MathLayoutLimits::default(),
               _BaseFontSize:   baseFontSize,
               _RootIndex:      rootIndex,
               _Boxes:          Stash::WithCapacity(256),
               _ChildIndices:   Stash::WithCapacity(512),
               _Occurrences:    Stash::WithCapacity(128),
               _Commands:       Stash::WithCapacity(512),
               _TotalTextBytes: 0, }
    }

    pub fn WithLimits(mut self, limits: MathLayoutLimits) -> Self
    {
        self._Limits = limits;
        self
    }

    fn CheckTextBudget(&mut self, text: &str) -> Result<(), String>
    {
        let len =
            u32::try_from(text.len()).map_err(|_| {
                                         "Total text byte budget exceeded in layout".to_string()
                                     })?;
        if len
           > self._Limits
                 ._MaxTotalTextBytes
                 .saturating_sub(self._TotalTextBytes)
        {
            return Err("Total text byte budget exceeded in layout".to_string());
        }
        self._TotalTextBytes += len;
        Ok(())
    }

    pub fn Build(mut self) -> Result<MathLayout, String>
    {
        if !self._BaseFontSize.is_finite() || self._BaseFontSize <= 0.0 {
            return Err("Math font size must be finite and positive".to_string());
        }
        let root = self._Document
                       .RootAt(self._RootIndex)
                       .ok_or_else(|| format!("Root index {} out of range", self._RootIndex))?;

        let rootBoxId = match &root._Kind {
            FrescoRootKind::Scalar { _ExprId } => {
                self.BuildExprBox(*_ExprId, self._BaseFontSize, 0, None)?
            }
            FrescoRootKind::Tensor { _Shape,
                                     _Elems,
                                     _Intent, } => {
                if let Some(intent) = _Intent {
                    self.BuildTensorOpIntent(root.Name(),
                                             _Shape,
                                             intent,
                                             _Elems,
                                             self._BaseFontSize)?
                } else {
                    self.BuildTensorGrid(root.Name(), _Shape, _Elems, self._BaseFontSize)?
                }
            }
        };

        // Measure layout bottom-up
        self.MeasureBox(rootBoxId);

        // Place layout top-down starting from (0, 0)
        let rootBox = &self._Boxes[rootBoxId];
        self.PlaceBox(rootBoxId, 0.0, rootBox._Ascent);

        let totalBounds = self._Boxes[rootBoxId]._AbsBounds;

        Ok(MathLayout { _Boxes:        self._Boxes.IntoBuff(),
                        _ChildIndices: self._ChildIndices.IntoBuff(),
                        _Occurrences:  self._Occurrences.IntoBuff(),
                        _Commands:     self._Commands.IntoBuff(),
                        _RootBox:      rootBoxId,
                        _TotalBounds:  totalBounds, })
    }

    fn RegisterOccurrence(&mut self, exprId: u32, coords: Option<MathCoordinates>)
                          -> Result<u32, String>
    {
        if self._Occurrences.Size() >= self._Limits._MaxOccurrences {
            return Err("Math layout occurrence limit exceeded".to_string());
        }
        let id = self._Occurrences.Size();
        self._Occurrences.Push(MathOccurrence { _Id:        id,
                                                _ExprId:    exprId,
                                                _Bounds:    Rect::Zero(),
                                                _RootIndex: self._RootIndex,
                                                _Coords:    coords, });
        return Ok(id);
    }

    fn AddBox(&mut self, kind: MathBoxKind, fontSize: f32, occId: u32) -> Result<u32, String>
    {
        if self._Boxes.Size() >= self._Limits._MaxBoxes {
            return Err("Math layout box limit exceeded".to_string());
        }
        let id = self._Boxes.Size();
        self._Boxes.Push(MathBox { _Kind:          kind,
                                   _Width:         0.0,
                                   _InkBounds:     Rect::Zero(),
                                   _Ascent:        0.0,
                                   _Descent:       0.0,
                                   _ChildrenStart: 0,
                                   _ChildrenCount: 0,
                                   _OccurrenceId:  occId,
                                   _RelX:          0.0,
                                   _RelY:          0.0,
                                   _AbsBounds:     Rect::Zero(),
                                   _FontSize:      fontSize, });
        Ok(id)
    }

    fn SetBoxChildren(&mut self, parentId: u32, children: Arr<'_, u32>)
    {
        let start = self._ChildIndices.Size();
        USeg::FromLen(children.Size()).Traverse(|i| self._ChildIndices.Push(children[i]));
        self._Boxes[parentId]._ChildrenStart = start;
        self._Boxes[parentId]._ChildrenCount = children.Size();
    }

    fn BuildTensorGrid(&mut self, _name: &str, shape: &crate::fresco::TensorShape,
                       elems: &Buff<u32>, fontSize: f32)
                       -> Result<u32, String>
    {
        let rank = shape.Rank();
        let (rows, cols) = if rank == 1 {
            (shape.Dim(0), 1u32)
        } else if rank == 2 {
            (shape.Dim(0), shape.Dim(1))
        } else {
            return Err("Higher-rank tensors require slice navigation; select a vector or matrix root".to_string());
        };

        let visibleRows = rows.min(self._Limits._MaxGridRows);
        let visibleCols = cols.min(self._Limits._MaxGridCols);
        let omittedRows = rows.saturating_sub(visibleRows);
        let omittedCols = cols.saturating_sub(visibleCols);

        if omittedRows > 0 {
            self.CheckTextBudget(&format!("... (+{} rows)", omittedRows))?;
        }
        if omittedCols > 0 {
            self.CheckTextBudget(&format!("... (+{} cols)", omittedCols))?;
        }
        let mut childBoxes = Stash::<u32>::WithCapacity(visibleRows * visibleCols);

        let mut r = 0u32;
        while r < visibleRows {
            let mut c = 0u32;
            while c < visibleCols {
                let flatIdx = r * cols + c;
                let exprId = if flatIdx < elems.Size() {
                    elems[flatIdx]
                } else {
                    u32::MAX
                };

                let semanticCoords = if rank == 1 {
                    Some(MathCoordinates::Vector(r))
                } else {
                    Some(MathCoordinates::Matrix(r, c))
                };

                let cellBox = if exprId != u32::MAX {
                    self.BuildExprBox(exprId, fontSize * 0.9, 0, semanticCoords)?
                } else {
                    self.CheckTextBudget("?")?;
                    let m = self._Metrics.MeasureAtom("?", fontSize, false, false);
                    self.AddBox(MathBoxKind::Atom { text:                    "?".to_string(),
                                                    isBold:                  false,
                                                    isItalic:                false,
                                                    color:                   MathColor::Muted,
                                                    paragraphBaselineOffset:
                                                        m.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?
                };
                childBoxes.Push(cellBox);
                c += 1;
            }
            r += 1;
        }

        let gridKind = MathBoxKind::Grid { rows: visibleRows,
                                           cols: visibleCols,
                                           omittedRows,
                                           omittedCols,
                                           colSpacing: 14.0,
                                           rowSpacing: 8.0,
                                           delimBracket: true,
                                           color: MathColor::Text };

        let gridBoxId = self.AddBox(gridKind, fontSize, u32::MAX)?;
        self.SetBoxChildren(gridBoxId, childBoxes.Arr());
        Ok(gridBoxId)
    }

    fn BuildTensorOpIntent(&mut self, name: &str, _shape: &crate::fresco::TensorShape,
                           intent: &crate::fenst::fresco_document::TensorOpIntent,
                           _elems: &Buff<u32>, fontSize: f32)
                           -> Result<u32, String>
    {
        use crate::fenst::fresco_document::TensorOpIntent;
        let occId = self.RegisterOccurrence(u32::MAX, None)?;
        let mut childBoxes = Stash::<u32>::WithCapacity(8);

        self.CheckTextBudget(name)?;
        let mName = self._Metrics.MeasureAtom(name, fontSize, true, true);
        let nameBox = self.AddBox(MathBoxKind::Atom { text:                    name.to_string(),
                                                      isBold:                  true,
                                                      isItalic:                true,
                                                      color:                   MathColor::Text,
                                                      paragraphBaselineOffset:
                                                          mName.paragraphBaselineOffset, },
                                  fontSize,
                                  occId)?;
        childBoxes.Push(nameBox);

        self.CheckTextBudget("=")?;
        let mEq = self._Metrics.MeasureAtom("=", fontSize, false, false);
        let eqBox = self.AddBox(MathBoxKind::Atom { text:                    "=".to_string(),
                                                    isBold:                  false,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        mEq.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;
        childBoxes.Push(eqBox);

        let (lhs, op, rhs) = match intent {
            TensorOpIntent::MatrixMultiply { lhsName, rhsName } => {
                (lhsName.as_str(), "·", rhsName.as_str())
            }
            TensorOpIntent::DotProduct { lhsName, rhsName } => {
                (lhsName.as_str(), "•", rhsName.as_str())
            }
            TensorOpIntent::HadamardProduct { lhsName, rhsName } => {
                (lhsName.as_str(), "⊙", rhsName.as_str())
            }
            TensorOpIntent::OuterProduct { lhsName, rhsName } => {
                (lhsName.as_str(), "⊗", rhsName.as_str())
            }
            TensorOpIntent::ElementwiseAdd { lhsName, rhsName } => {
                (lhsName.as_str(), "+", rhsName.as_str())
            }
            TensorOpIntent::ElementwiseSubtract { lhsName, rhsName } => {
                (lhsName.as_str(), "−", rhsName.as_str())
            }
            TensorOpIntent::ElementwiseDivide { lhsName, rhsName } => {
                (lhsName.as_str(), "/", rhsName.as_str())
            }
            TensorOpIntent::Scale { tensorName, .. } => ("s", "·", tensorName.as_str()),
        };

        self.CheckTextBudget(lhs)?;
        let mLhs = self._Metrics.MeasureAtom(lhs, fontSize, true, true);
        let lhsBox = self.AddBox(MathBoxKind::Atom { text:                    lhs.to_string(),
                                                     isBold:                  true,
                                                     isItalic:                true,
                                                     color:                   MathColor::Text,
                                                     paragraphBaselineOffset:
                                                         mLhs.paragraphBaselineOffset, },
                                 fontSize,
                                 occId)?;
        childBoxes.Push(lhsBox);

        self.CheckTextBudget(op)?;
        let mOp = self._Metrics.MeasureAtom(op, fontSize, false, false);
        let opBox = self.AddBox(MathBoxKind::Atom { text:                    op.to_string(),
                                                    isBold:                  false,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        mOp.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;
        childBoxes.Push(opBox);

        self.CheckTextBudget(rhs)?;
        let mRhs = self._Metrics.MeasureAtom(rhs, fontSize, true, true);
        let rhsBox = self.AddBox(MathBoxKind::Atom { text:                    rhs.to_string(),
                                                     isBold:                  true,
                                                     isItalic:                true,
                                                     color:                   MathColor::Text,
                                                     paragraphBaselineOffset:
                                                         mRhs.paragraphBaselineOffset, },
                                 fontSize,
                                 occId)?;
        childBoxes.Push(rhsBox);

        let rowBoxId = self.AddBox(MathBoxKind::Row { spacing: 6.0 }, fontSize, occId)?;
        self.SetBoxChildren(rowBoxId, childBoxes.Arr());
        return Ok(rowBoxId);
    }

    fn BuildExprBox(&mut self, exprId: u32, fontSize: f32, depth: u32,
                    coords: Option<MathCoordinates>)
                    -> Result<u32, String>
    {
        if depth > self._Limits._MaxDepth {
            return Err("Expression depth limit exceeded in layout".to_string());
        }

        let repos = self._Document.Repos();
        if !repos.IsValidExprId(exprId) {
            return Err(format!("Invalid expression ID {}", exprId));
        }

        let occId = self.RegisterOccurrence(exprId, coords)?;

        let view = match repos.Inspect(exprId) {
            Ok(v) => v,
            Err(ExprReadError::EmptyNode(_)) => {
                return Err(format!("Empty expression at ID {}", exprId));
            }
            Err(ExprReadError::InvalidId(_)) => {
                return Err(format!("Invalid expression ID {}", exprId));
            }
            Err(ExprReadError::UnsupportedNode(_)) => {
                return Err(format!("Unsupported expression node at ID {}", exprId));
            }
        };

        match view {
            ExprView::Real(val) => {
                let formatted = if val != 0.0 && (val.abs() < 1e-4 || val.abs() >= 1e9) {
                    format!("{:e}", val)
                } else {
                    val.to_string()
                };
                self.CheckTextBudget(&formatted)?;
                let m = self._Metrics
                            .MeasureAtom(&formatted, fontSize, false, false);
                let atom = self.AddBox(MathBoxKind::Atom { text:                    formatted,
                                                           isBold:                  false,
                                                           isItalic:                false,
                                                           color:                   MathColor::Text,
                                                           paragraphBaselineOffset:
                                                               m.paragraphBaselineOffset, },
                                       fontSize,
                                       occId)?;
                return Ok(atom);
            }

            ExprView::Var(varIdx) => {
                let name = repos.VarNameAt(varIdx);
                if name.len() > self._Limits._MaxIdentifierLen as usize {
                    return Err(format!("Variable identifier exceeds maximum allowed length of {} bytes",
                                       self._Limits._MaxIdentifierLen));
                }
                self.CheckTextBudget(name)?;
                let isTensorSubvar = !repos.VarAttrAt(varIdx).Shape().IsScalar();
                let m = self._Metrics.MeasureAtom(name, fontSize, false, true);
                let atom = self.AddBox(MathBoxKind::Atom { text:
                                                               name.to_string(),
                                                           isBold:                  isTensorSubvar,
                                                           isItalic:                true,
                                                           color:                   MathColor::Text,
                                                           paragraphBaselineOffset:
                                                               m.paragraphBaselineOffset, },
                                       fontSize,
                                       occId)?;
                return Ok(atom);
            }

            ExprView::Sum(poly) => {
                let coSize = poly.CommonSize();
                let totalChild = poly.SizeChild();

                let mut children = Stash::New();
                let mut i = 0u32;
                while i < coSize {
                    if i > 0 {
                        self.CheckTextBudget("+")?;
                        let m = self._Metrics.MeasureAtom("+", fontSize, false, false);
                        let plus = self.AddBox(MathBoxKind::Atom { text:
                                                                       "+".to_string(),
                                                                   isBold:                  false,
                                                                   isItalic:                false,
                                                                   color:
                                                                       MathColor::Operator,
                                                                   paragraphBaselineOffset:
                                                                       m.paragraphBaselineOffset, },
                                               fontSize,
                                               u32::MAX)?;
                        children.Push(plus);
                    }
                    let cBox = self.BuildExprBox(poly.Child(i), fontSize, depth + 1, coords)?;
                    let cBox = if self.NeedsParensInSum(poly.Child(i)) {
                        self.WrapInDelimiters(cBox,
                                              DelimiterKind::ParenthesisLeft,
                                              DelimiterKind::ParenthesisRight,
                                              fontSize)?
                    } else {
                        cBox
                    };
                    children.Push(cBox);
                    i += 1;
                }

                while i < totalChild {
                    self.CheckTextBudget("-")?;
                    let m = self._Metrics.MeasureAtom("-", fontSize, false, false);
                    let minus = self.AddBox(MathBoxKind::Atom { text:
                                                                    "-".to_string(),
                                                                isBold:                  false,
                                                                isItalic:                false,
                                                                color:
                                                                    MathColor::Operator,
                                                                paragraphBaselineOffset:
                                                                    m.paragraphBaselineOffset, },
                                            fontSize,
                                            u32::MAX)?;
                    children.Push(minus);

                    let subExprId = poly.Child(i);
                    let subBox = self.BuildExprBox(subExprId, fontSize, depth + 1, coords)?;

                    // If subtracting a SumExpr, parenthesize to preserve a - (b - c)
                    let subIsSum = self.NeedsParensInSum(subExprId);

                    if subIsSum {
                        let paren = self.WrapInDelimiters(subBox,
                                                          DelimiterKind::ParenthesisLeft,
                                                          DelimiterKind::ParenthesisRight,
                                                          fontSize)?;
                        children.Push(paren);
                    } else {
                        children.Push(subBox);
                    }
                    i += 1;
                }

                if children.Size() == 0 {
                    self.CheckTextBudget("0")?;
                    let m = self._Metrics.MeasureAtom("0", fontSize, false, false);
                    return self.AddBox(MathBoxKind::Atom { text:
                                                                   "0".to_string(),
                                                               isBold:                  false,
                                                               isItalic:                false,
                                                               color:
                                                                   MathColor::Text,
                                                               paragraphBaselineOffset:
                                                                   m.paragraphBaselineOffset, },
                                           fontSize,
                                           occId);
                }
                let row = self.AddBox(MathBoxKind::Row { spacing: 4.0 }, fontSize, occId)?;
                self.SetBoxChildren(row, children.Arr());
                return Ok(row);
            }

            ExprView::Prod(poly) => {
                let coSize = poly.CommonSize();
                let totalChild = poly.SizeChild();

                // Numerator factors: 0..coSize
                let numerBox = if coSize == 0 {
                    self.CheckTextBudget("1")?;
                    let m = self._Metrics.MeasureAtom("1", fontSize, false, false);
                    self.AddBox(MathBoxKind::Atom { text:                    "1".to_string(),
                                                    isBold:                  false,
                                                    isItalic:                false,
                                                    color:                   MathColor::Text,
                                                    paragraphBaselineOffset:
                                                        m.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?
                } else if coSize == 1 {
                    let factorId = poly.Child(0);
                    let factorBox = self.BuildExprBox(factorId, fontSize, depth + 1, coords)?;
                    if self.NeedsParensInProduct(factorId) {
                        self.WrapInDelimiters(factorBox,
                                              DelimiterKind::ParenthesisLeft,
                                              DelimiterKind::ParenthesisRight,
                                              fontSize)?
                    } else {
                        factorBox
                    }
                } else {
                    let mut numChildren = Stash::New();
                    let mut k = 0u32;
                    while k < coSize {
                        if k > 0 {
                            self.CheckTextBudget("\u{00b7}")?;
                            let m = self._Metrics
                                        .MeasureAtom("\u{00b7}", fontSize, false, false);
                            let sp = self.AddBox(MathBoxKind::Atom { text:
                                                                     "\u{00b7}".to_string(),
                                                                 isBold:                  false,
                                                                 isItalic:                false,
                                                                 color:
                                                                     MathColor::Text,
                                                                 paragraphBaselineOffset:
                                                                     m.paragraphBaselineOffset, },
                                             fontSize,
                                             u32::MAX)?;
                            numChildren.Push(sp);
                        }
                        let factorId = poly.Child(k);
                        let factorBox = self.BuildExprBox(factorId, fontSize, depth + 1, coords)?;
                        if self.NeedsParensInProduct(factorId) {
                            let p = self.WrapInDelimiters(factorBox,
                                                          DelimiterKind::ParenthesisLeft,
                                                          DelimiterKind::ParenthesisRight,
                                                          fontSize)?;
                            numChildren.Push(p);
                        } else {
                            numChildren.Push(factorBox);
                        }
                        k += 1;
                    }
                    let r = self.AddBox(MathBoxKind::Row { spacing: 2.0 }, fontSize, u32::MAX)?;
                    self.SetBoxChildren(r, numChildren.Arr());
                    r
                };

                // If no denominator factors, return numerator directly
                if coSize == totalChild {
                    let row = self.AddBox(MathBoxKind::Row { spacing: 0.0 }, fontSize, occId)?;
                    self.SetBoxChildren(row, Arr::from(&[numerBox]));
                    return Ok(row);
                }

                // Denominator factors: coSize..totalChild
                let denomCount = totalChild - coSize;
                let denomBox = if denomCount == 1 {
                    let factorId = poly.Child(coSize);
                    let factorBox = self.BuildExprBox(factorId, fontSize, depth + 1, coords)?;
                    if self.NeedsParensInProduct(factorId) {
                        self.WrapInDelimiters(factorBox,
                                              DelimiterKind::ParenthesisLeft,
                                              DelimiterKind::ParenthesisRight,
                                              fontSize)?
                    } else {
                        factorBox
                    }
                } else {
                    let mut denChildren = Stash::New();
                    let mut k = coSize;
                    while k < totalChild {
                        if k > coSize {
                            self.CheckTextBudget("\u{00b7}")?;
                            let m = self._Metrics
                                        .MeasureAtom("\u{00b7}", fontSize, false, false);
                            let sp = self.AddBox(MathBoxKind::Atom { text:
                                                                     "\u{00b7}".to_string(),
                                                                 isBold:                  false,
                                                                 isItalic:                false,
                                                                 color:
                                                                     MathColor::Text,
                                                                 paragraphBaselineOffset:
                                                                     m.paragraphBaselineOffset, },
                                             fontSize,
                                             u32::MAX)?;
                            denChildren.Push(sp);
                        }
                        let factorId = poly.Child(k);
                        let factorBox = self.BuildExprBox(factorId, fontSize, depth + 1, coords)?;
                        if self.NeedsParensInProduct(factorId) {
                            let p = self.WrapInDelimiters(factorBox,
                                                          DelimiterKind::ParenthesisLeft,
                                                          DelimiterKind::ParenthesisRight,
                                                          fontSize)?;
                            denChildren.Push(p);
                        } else {
                            denChildren.Push(factorBox);
                        }
                        k += 1;
                    }
                    let r = self.AddBox(MathBoxKind::Row { spacing: 2.0 }, fontSize, u32::MAX)?;
                    self.SetBoxChildren(r, denChildren.Arr());
                    r
                };

                let frac =
                    self.AddBox(MathBoxKind::Fraction { ruleThickness: (fontSize * 0.08).max(1.5),
                                                        gap:           (fontSize * 0.2).max(3.0),
                                                        color:         MathColor::Text, },
                                fontSize,
                                occId)?;
                self.SetBoxChildren(frac, Arr::from(&[numerBox, denomBox]));
                return Ok(frac);
            }

            ExprView::Pow(poly) => {
                if poly.SizeChild() != 2 || poly.CommonSize() != 1 {
                    return Err(format!("Invalid power operands at expression ID {}", exprId));
                }
                let baseId = poly.Child(0);
                let expId = poly.Child(1);

                let baseBox = self.BuildExprBox(baseId, fontSize, depth + 1, coords)?;
                let baseFinal = if self.NeedsParensInBase(baseId) {
                    self.WrapInDelimiters(baseBox,
                                          DelimiterKind::ParenthesisLeft,
                                          DelimiterKind::ParenthesisRight,
                                          fontSize)?
                } else {
                    baseBox
                };

                let expBox = self.BuildExprBox(expId, fontSize * 0.72, depth + 1, coords)?;

                let script = self.AddBox(MathBoxKind::Script { hasSub: false,
                                                               hasSup: true, },
                                         fontSize,
                                         occId)?;
                self.SetBoxChildren(script, Arr::from(&[baseFinal, expBox]));
                return Ok(script);
            }

            ExprView::Func { name: fnName,
                             arg: argId, } => {
                if fnName.len() > self._Limits._MaxIdentifierLen as usize {
                    return Err(format!("Function identifier exceeds maximum allowed length of {} bytes",
                                       self._Limits._MaxIdentifierLen));
                }
                self.CheckTextBudget(fnName)?;
                let m = self._Metrics.MeasureAtom(fnName, fontSize, false, false);
                let fnAtom =
                    self.AddBox(MathBoxKind::Atom { text:                    fnName.to_string(),
                                                    isBold:                  false,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        m.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;

                let argBox = self.BuildExprBox(argId, fontSize, depth + 1, coords)?;
                let parenArg = self.WrapInDelimiters(argBox,
                                                     DelimiterKind::ParenthesisLeft,
                                                     DelimiterKind::ParenthesisRight,
                                                     fontSize)?;

                let row = self.AddBox(MathBoxKind::Row { spacing: 2.0 }, fontSize, occId)?;
                self.SetBoxChildren(row, Arr::from(&[fnAtom, parenArg]));
                return Ok(row);
            }

            ExprView::Cond { condition,
                             ifTrue,
                             ifFalse, } => {
                let cBox = self.BuildExprBox(condition, fontSize, depth + 1, coords)?;
                let tBox = self.BuildExprBox(ifTrue, fontSize, depth + 1, coords)?;
                let fBox = self.BuildExprBox(ifFalse, fontSize, depth + 1, coords)?;

                self.CheckTextBudget("if ")?;
                let mIf = self._Metrics.MeasureAtom("if ", fontSize, false, false);
                let ifAtom =
                    self.AddBox(MathBoxKind::Atom { text:                    "if ".to_string(),
                                                    isBold:                  true,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        mIf.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;

                self.CheckTextBudget(" then ")?;
                let mThen = self._Metrics.MeasureAtom(" then ", fontSize, false, false);
                let thenAtom =
                    self.AddBox(MathBoxKind::Atom { text:                    " then ".to_string(),
                                                    isBold:                  true,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        mThen.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;

                self.CheckTextBudget(" else ")?;
                let mElse = self._Metrics.MeasureAtom(" else ", fontSize, false, false);
                let elseAtom =
                    self.AddBox(MathBoxKind::Atom { text:                    " else ".to_string(),
                                                    isBold:                  true,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        mElse.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;

                let row = self.AddBox(MathBoxKind::Row { spacing: 2.0 }, fontSize, occId)?;
                self.SetBoxChildren(row,
                                    Arr::from(&[ifAtom, cBox, thenAtom, tBox, elseAtom, fBox]));
                return Ok(row);
            }

            ExprView::Logic { op,
                              left: leftId,
                              right: rightId, } => {
                let isUnary = op == LogicOp::Not;
                let opStr = match op {
                    LogicOp::Lt => " < ",
                    LogicOp::Le => " \u{2264} ",
                    LogicOp::Gt => " > ",
                    LogicOp::Ge => " \u{2265} ",
                    LogicOp::Eq => " == ",
                    LogicOp::Ne => " \u{2260} ",
                    LogicOp::And => " && ",
                    LogicOp::Or => " || ",
                    LogicOp::Not => "!",
                };
                self.CheckTextBudget(opStr)?;
                let m = self._Metrics.MeasureAtom(opStr, fontSize, true, false);
                let opAtom =
                    self.AddBox(MathBoxKind::Atom { text:                    opStr.to_string(),
                                                    isBold:                  true,
                                                    isItalic:                false,
                                                    color:                   MathColor::Operator,
                                                    paragraphBaselineOffset:
                                                        m.paragraphBaselineOffset, },
                                fontSize,
                                u32::MAX)?;
                let left = self.BuildExprBox(leftId, fontSize, depth + 1, coords)?;
                let left = if self.NeedsParensInLogic(leftId, isUnary) {
                    self.WrapInDelimiters(left,
                                          DelimiterKind::ParenthesisLeft,
                                          DelimiterKind::ParenthesisRight,
                                          fontSize)?
                } else {
                    left
                };
                let row = self.AddBox(MathBoxKind::Row { spacing: 2.0 }, fontSize, occId)?;
                if isUnary {
                    self.SetBoxChildren(row, Arr::from(&[opAtom, left]));
                } else {
                    let right = self.BuildExprBox(rightId, fontSize, depth + 1, coords)?;
                    let right = if self.NeedsParensInLogic(rightId, false) {
                        self.WrapInDelimiters(right,
                                              DelimiterKind::ParenthesisLeft,
                                              DelimiterKind::ParenthesisRight,
                                              fontSize)?
                    } else {
                        right
                    };
                    self.SetBoxChildren(row, Arr::from(&[left, opAtom, right]));
                }
                return Ok(row);
            }
        }
    }

    fn NeedsParensInSum(&self, exprId: u32) -> bool
    {
        let repos = self._Document.Repos();
        matches!(repos.Inspect(exprId),
                 Ok(ExprView::Sum(_) | ExprView::Logic { .. } | ExprView::Cond { .. }))
    }

    fn NeedsParensInLogic(&self, exprId: u32, unary: bool) -> bool
    {
        let repos = self._Document.Repos();
        let Ok(view) = repos.Inspect(exprId) else {
            return false;
        };
        match view {
            ExprView::Logic { .. } | ExprView::Cond { .. } => true,
            ExprView::Sum(_) | ExprView::Prod(_) | ExprView::Pow(_) if unary => true,
            _ => false,
        }
    }

    fn NeedsParensInProduct(&self, exprId: u32) -> bool
    {
        let repos = self._Document.Repos();
        let Ok(view) = repos.Inspect(exprId) else {
            return false;
        };
        match view {
            ExprView::Sum(_)
            | ExprView::Prod(_)
            | ExprView::Cond { .. }
            | ExprView::Logic { .. } => true,
            ExprView::Real(val) => val.is_sign_negative(),
            _ => false,
        }
    }

    fn NeedsParensInBase(&self, exprId: u32) -> bool
    {
        let repos = self._Document.Repos();
        let Ok(view) = repos.Inspect(exprId) else {
            return false;
        };
        match view {
            ExprView::Sum(_)
            | ExprView::Prod(_)
            | ExprView::Pow(_)
            | ExprView::Logic { .. }
            | ExprView::Cond { .. } => true,
            ExprView::Real(val) => val.is_sign_negative(),
            _ => false,
        }
    }

    fn WrapInDelimiters(&mut self, innerBox: u32, left: DelimiterKind, right: DelimiterKind,
                        fontSize: f32)
                        -> Result<u32, String>
    {
        let delimBox = self.AddBox(MathBoxKind::Delimiter { kindLeft:  left,
                                                            kindRight: right,
                                                            thickness: (fontSize * 0.08).max(1.2),
                                                            color:     MathColor::Text, },
                                   fontSize,
                                   u32::MAX)?;
        self.SetBoxChildren(delimBox, Arr::from(&[innerBox]));
        Ok(delimBox)
    }

    fn MeasureBox(&mut self, boxId: u32)
    {
        // Bottom-up: measure all children first
        let start = self._Boxes[boxId]._ChildrenStart;
        let count = self._Boxes[boxId]._ChildrenCount;
        let mut i = 0u32;
        while i < count {
            let childId = self._ChildIndices[start + i];
            self.MeasureBox(childId);
            i += 1;
        }

        let b = &self._Boxes[boxId];
        let fontSize = b._FontSize;

        match &b._Kind {
            MathBoxKind::Atom { text,
                                isBold,
                                isItalic,
                                .. } => {
                let m = self._Metrics
                            .MeasureAtom(text, fontSize, *isBold, *isItalic);
                let bMut = &mut self._Boxes[boxId];
                bMut._Width = m.width;
                bMut._InkBounds = m.inkBounds;
                bMut._Ascent = m.ascent;
                bMut._Descent = m.descent;
                if let MathBoxKind::Atom { paragraphBaselineOffset,
                                           .. } = &mut bMut._Kind
                {
                    *paragraphBaselineOffset = m.paragraphBaselineOffset;
                }
            }

            MathBoxKind::Row { spacing } => {
                let sp = *spacing;
                let mut curX = 0.0f32;
                let mut maxAscent = 0.0f32;
                let mut maxDescent = 0.0f32;

                let mut k = 0u32;
                while k < count {
                    let childId = self._ChildIndices[start + k];
                    let child = &self._Boxes[childId];
                    if k > 0 {
                        curX += sp;
                    }
                    curX += child._Width;
                    maxAscent = maxAscent.max(child._Ascent);
                    maxDescent = maxDescent.max(child._Descent);
                    k += 1;
                }

                let totalW = curX;
                let bMut = &mut self._Boxes[boxId];
                bMut._Width = totalW;
                bMut._Ascent = maxAscent;
                bMut._Descent = maxDescent;
            }

            MathBoxKind::Fraction { ruleThickness, gap, .. } => {
                let numId = self._ChildIndices[start];
                let denId = self._ChildIndices[start + 1];
                let num = &self._Boxes[numId];
                let den = &self._Boxes[denId];

                let pad = 6.0f32;
                let contentW = num._Width.max(den._Width);
                let w = contentW + 2.0 * pad;

                let axisH = fontSize * 0.25; // standard fraction bar axis height above baseline
                let t = *ruleThickness;
                let g = *gap;

                let ascent = axisH + t * 0.5 + g + num._Ascent + num._Descent;
                let descent = (g + t * 0.5 + den._Ascent + den._Descent - axisH).max(0.0);

                let bMut = &mut self._Boxes[boxId];
                bMut._Width = w;
                bMut._Ascent = ascent;
                bMut._Descent = descent;
            }

            MathBoxKind::Script { hasSub, hasSup } => {
                let baseId = self._ChildIndices[start];
                let base = &self._Boxes[baseId];

                let mut w = base._Width;
                let mut ascent = base._Ascent;
                let mut descent = base._Descent;

                if *hasSup && count > 1 {
                    let supId = self._ChildIndices[start + 1];
                    let sup = &self._Boxes[supId];
                    let supShift = base._Ascent * 0.5;
                    w += sup._Width + 2.0;
                    ascent = ascent.max(sup._Ascent + supShift);
                    descent = descent.max(sup._Descent - supShift);
                }

                if *hasSub && count > 1 {
                    let subId = if *hasSup && count > 2 {
                        self._ChildIndices[start + 2]
                    } else {
                        self._ChildIndices[start + 1]
                    };
                    let sub = &self._Boxes[subId];
                    let subShift = base._Descent * 0.4;
                    w += sub._Width + 2.0;
                    descent = descent.max(sub._Descent + subShift);
                    ascent = ascent.max(sub._Ascent - subShift);
                }

                let bMut = &mut self._Boxes[boxId];
                bMut._Width = w;
                bMut._Ascent = ascent;
                bMut._Descent = descent;
            }

            MathBoxKind::Delimiter { .. } => {
                let innerId = self._ChildIndices[start];
                let innerW = self._Boxes[innerId]._Width;
                let innerAsc = self._Boxes[innerId]._Ascent;
                let innerDesc = self._Boxes[innerId]._Descent;

                let delimW = (fontSize * 0.45).max(7.0);
                let pad = 3.0f32;

                let bMut = &mut self._Boxes[boxId];
                let w = innerW + 2.0 * delimW + 2.0 * pad;
                bMut._Width = w;
                bMut._Ascent = innerAsc + pad;
                bMut._Descent = innerDesc + pad;
            }

            MathBoxKind::Grid { rows,
                                cols,
                                omittedRows,
                                omittedCols,
                                colSpacing,
                                rowSpacing,
                                delimBracket,
                                .. } => {
                let rCount = *rows;
                let cCount = *cols;
                let colSp = *colSpacing;
                let rowSp = *rowSpacing;

                let mut colWidths = Buff::FromDispenser(cCount, |_| 0.0f32);
                let mut rowAscents = Buff::FromDispenser(rCount, |_| 0.0f32);
                let mut rowDescents = Buff::FromDispenser(rCount, |_| 0.0f32);

                let mut r = 0u32;
                while r < rCount {
                    let mut c = 0u32;
                    while c < cCount {
                        let idx = r * cCount + c;
                        if idx < count {
                            let cellId = self._ChildIndices[start + idx];
                            let cell = &self._Boxes[cellId];
                            colWidths[c] = colWidths[c].max(cell._Width);
                            rowAscents[r] = rowAscents[r].max(cell._Ascent);
                            rowDescents[r] = rowDescents[r].max(cell._Descent);
                        }
                        c += 1;
                    }
                    r += 1;
                }

                let mut totalW = 0.0f32;
                let mut c = 0u32;
                while c < colWidths.Size() {
                    if c > 0 {
                        totalW += colSp;
                    }
                    totalW += colWidths[c];
                    c += 1;
                }

                if *omittedCols > 0 {
                    let m = self._Metrics
                                .MeasureAtom(&format!("... (+{} cols)", omittedCols),
                                             fontSize,
                                             false,
                                             false);
                    totalW += m.width + colSp;
                }

                let mut totalH = 8.0f32;
                let mut rIdx = 0u32;
                while rIdx < rowAscents.Size() {
                    if rIdx > 0 {
                        totalH += rowSp;
                    }
                    totalH += rowAscents[rIdx] + rowDescents[rIdx];
                    rIdx += 1;
                }

                if *omittedRows > 0 {
                    let label = format!("... (+{} rows)", omittedRows);
                    let m = self._Metrics
                                .MeasureAtom(&label, fontSize * 0.8, false, true);
                    totalW = totalW.max(m.width + 4.0);
                    totalH += m.ascent + m.descent + rowSp;
                }

                let delimPad = if *delimBracket {
                    (fontSize * 0.5).max(8.0)
                } else {
                    0.0
                };

                let bMut = &mut self._Boxes[boxId];
                let w = totalW + 2.0 * delimPad + 8.0;
                bMut._Width = w;
                bMut._Ascent = totalH * 0.5 + fontSize * 0.2;
                bMut._Descent = (totalH * 0.5 - fontSize * 0.2).max(0.0);
            }
        }
    }

    fn PlaceBox(&mut self, boxId: u32, absX: f32, absBaselineY: f32)
    {
        let start = self._Boxes[boxId]._ChildrenStart;
        let count = self._Boxes[boxId]._ChildrenCount;

        let kind = self._Boxes[boxId]._Kind.clone();
        let fontSize = self._Boxes[boxId]._FontSize;
        let ascent = self._Boxes[boxId]._Ascent;
        let descent = self._Boxes[boxId]._Descent;
        let width = self._Boxes[boxId]._Width;
        let occId = self._Boxes[boxId]._OccurrenceId;

        let absTop = absBaselineY - ascent;
        let bounds = Rect::New(absX, absTop, width, ascent + descent);
        self._Boxes[boxId]._AbsBounds = bounds;

        match kind {
            MathBoxKind::Atom { text,
                                isBold,
                                isItalic,
                                color,
                                paragraphBaselineOffset, } => {
                self._Commands.Push(MathDrawCommand::Text { text,
                                                            x: absX,
                                                            yBaseline: absBaselineY,
                                                            paragraphBaselineOffset,
                                                            fontSize,
                                                            isBold,
                                                            isItalic,
                                                            color });
            }

            MathBoxKind::Row { spacing } => {
                let mut curX = absX;
                let mut k = 0u32;
                while k < count {
                    let childId = self._ChildIndices[start + k];
                    let childW = self._Boxes[childId]._Width;
                    self.PlaceBox(childId, curX, absBaselineY);
                    curX += childW + spacing;
                    k += 1;
                }
            }

            MathBoxKind::Fraction { ruleThickness,
                                    gap,
                                    color, } => {
                let numId = self._ChildIndices[start];
                let denId = self._ChildIndices[start + 1];

                let ruleY = absBaselineY - fontSize * 0.25 - ruleThickness * 0.5;
                let ruleBounds = Rect::New(absX + 2.0, ruleY, width - 4.0, ruleThickness);
                self._Commands
                    .Push(MathDrawCommand::Rule { bounds: ruleBounds,
                                                  color });

                let numW = self._Boxes[numId]._Width;
                let denW = self._Boxes[denId]._Width;

                let numX = absX + (width - numW) * 0.5;
                let denX = absX + (width - denW) * 0.5;

                let numBaseline = ruleY - gap - self._Boxes[numId]._Descent;
                let denBaseline = ruleY + ruleThickness + gap + self._Boxes[denId]._Ascent;

                self.PlaceBox(numId, numX, numBaseline);
                self.PlaceBox(denId, denX, denBaseline);
            }

            MathBoxKind::Script { hasSub, hasSup } => {
                let baseId = self._ChildIndices[start];
                let baseW = self._Boxes[baseId]._Width;
                self.PlaceBox(baseId, absX, absBaselineY);

                let scriptX = absX + baseW + 2.0;

                if hasSup && count > 1 {
                    let supId = self._ChildIndices[start + 1];
                    let supShift = self._Boxes[baseId]._Ascent * 0.5;
                    self.PlaceBox(supId, scriptX, absBaselineY - supShift);
                }

                if hasSub && count > 1 {
                    let subId = if hasSup && count > 2 {
                        self._ChildIndices[start + 2]
                    } else {
                        self._ChildIndices[start + 1]
                    };
                    let subShift = self._Boxes[baseId]._Descent * 0.4;
                    self.PlaceBox(subId, scriptX, absBaselineY + subShift);
                }
            }

            MathBoxKind::Delimiter { kindLeft,
                                     kindRight,
                                     thickness,
                                     color, } => {
                let innerId = self._ChildIndices[start];
                let innerW = self._Boxes[innerId]._Width;
                let delimW = (fontSize * 0.45).max(7.0);
                let pad = 3.0f32;

                let leftBounds = Rect::New(absX, bounds.y, delimW, bounds.height);
                self._Commands
                    .Push(MathDrawCommand::Delimiter { kind: kindLeft,
                                                       bounds: leftBounds,
                                                       thickness,
                                                       color });

                let innerX = absX + delimW + pad;
                self.PlaceBox(innerId, innerX, absBaselineY);

                let rightX = innerX + innerW + pad;
                let rightBounds = Rect::New(rightX, bounds.y, delimW, bounds.height);
                self._Commands
                    .Push(MathDrawCommand::Delimiter { kind: kindRight,
                                                       bounds: rightBounds,
                                                       thickness,
                                                       color });
            }

            MathBoxKind::Grid { rows,
                                cols,
                                omittedRows,
                                omittedCols,
                                colSpacing,
                                rowSpacing,
                                delimBracket,
                                color, } => {
                let delimW = if delimBracket {
                    (fontSize * 0.5).max(8.0)
                } else {
                    0.0
                };

                if delimBracket {
                    self._Commands.Push(MathDrawCommand::Delimiter { kind:
                                                                         DelimiterKind::BracketLeft,
                                                                     bounds:
                                                                         Rect::New(absX,
                                                                                   bounds.y,
                                                                                   delimW,
                                                                                   bounds.height),
                                                                     thickness: 1.5,
                                                                     color });
                    self._Commands.Push(MathDrawCommand::Delimiter {
                        kind:      DelimiterKind::BracketRight,
                        bounds:    Rect::New(absX + width - delimW, bounds.y, delimW, bounds.height),
                        thickness: 1.5,
                        color,
                    });
                }

                // Compute column widths and row heights again for placement
                let mut colWidths = Buff::FromDispenser(cols, |_| 0.0f32);
                let mut rowAscents = Buff::FromDispenser(rows, |_| 0.0f32);
                let mut rowDescents = Buff::FromDispenser(rows, |_| 0.0f32);

                let mut r = 0u32;
                while r < rows {
                    let mut c = 0u32;
                    while c < cols {
                        let idx = r * cols + c;
                        if idx < count {
                            let cellId = self._ChildIndices[start + idx];
                            let cell = &self._Boxes[cellId];
                            colWidths[c] = colWidths[c].max(cell._Width);
                            rowAscents[r] = rowAscents[r].max(cell._Ascent);
                            rowDescents[r] = rowDescents[r].max(cell._Descent);
                        }
                        c += 1;
                    }
                    r += 1;
                }

                let mut curY = bounds.y + 4.0;
                let mut rIdx = 0u32;
                while rIdx < rows {
                    let rowAsc = rowAscents[rIdx];
                    let rowDesc = rowDescents[rIdx];
                    let cellBaselineY = curY + rowAsc;

                    let mut curX = absX + delimW + 4.0;
                    let mut cIdx = 0u32;
                    while cIdx < cols {
                        let idx = rIdx * cols + cIdx;
                        let cellW = colWidths[cIdx];
                        if idx < count {
                            let cellId = self._ChildIndices[start + idx];
                            let cActualW = self._Boxes[cellId]._Width;
                            // Center horizontally in column
                            let cellX = curX + (cellW - cActualW) * 0.5;
                            self.PlaceBox(cellId, cellX, cellBaselineY);
                        }
                        curX += cellW + colSpacing;
                        cIdx += 1;
                    }

                    if omittedCols > 0 && rIdx == rows / 2 {
                        let omittedLabel = format!("... (+{} cols)", omittedCols);
                        let m = self._Metrics
                                    .MeasureAtom(&omittedLabel, fontSize, false, false);
                        let ink = m.inkBounds;
                        let visual =
                            Rect::New(curX + ink.x, cellBaselineY + ink.y, ink.width, ink.height);
                        self._Boxes[boxId]._AbsBounds =
                            self._Boxes[boxId]._AbsBounds.Union(&visual);
                        self._Commands
                            .Push(MathDrawCommand::Text { text: omittedLabel,
                                                          x: curX,
                                                          yBaseline: cellBaselineY,
                                                          paragraphBaselineOffset:
                                                              m.paragraphBaselineOffset,
                                                          fontSize,
                                                          isBold: false,
                                                          isItalic: false,
                                                          color: MathColor::Muted });
                    }

                    curY += rowAsc + rowDesc + rowSpacing;
                    rIdx += 1;
                }

                if omittedRows > 0 {
                    let omittedLabel = format!("... (+{} rows)", omittedRows);
                    let m = self._Metrics
                                .MeasureAtom(&omittedLabel, fontSize * 0.8, false, true);
                    let ink = m.inkBounds;
                    let visual = Rect::New(absX + delimW + 8.0 + ink.x,
                                           curY + m.ascent + ink.y,
                                           ink.width,
                                           ink.height);
                    self._Boxes[boxId]._AbsBounds = self._Boxes[boxId]._AbsBounds.Union(&visual);
                    self._Commands.Push(MathDrawCommand::Text { text:
                                                                    omittedLabel,
                                                                x:                       absX
                                                                                         + delimW
                                                                                         + 8.0,
                                                                yBaseline:               curY
                                                                                         + m.ascent,
                                                                paragraphBaselineOffset:
                                                                    m.paragraphBaselineOffset,
                                                                fontSize:                fontSize
                                                                                         * 0.8,
                                                                isBold:                  false,
                                                                isItalic:                true,
                                                                color:
                                                                    MathColor::Muted, });
                }
            }
        }

        let ink = self._Boxes[boxId]._InkBounds;
        let mut visual = self._Boxes[boxId]._AbsBounds.Union(&Rect::New(absX + ink.x,
                                                                        absBaselineY + ink.y,
                                                                        ink.width,
                                                                        ink.height));
        USeg::FromLen(count).Traverse(|i| {
                                let child = self._ChildIndices[start + i];
                                visual = visual.Union(&self._Boxes[child]._AbsBounds);
                            });
        self._Boxes[boxId]._AbsBounds = visual;
        if occId != u32::MAX {
            self._Occurrences[occId]._Bounds = visual;
        }
    }
}
