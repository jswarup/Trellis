//-- cask.rs -----------------------------------------------------------------------------------------------------------
//! 2D UI layout hierarchy and Clay-style layout engine using iterative depth-first traversal.
//!
//! Provides the core layout data structures (`Cask`, `Rect`, `Padding`, `Sizing`, `CaskRenderCommand`)
//! and 2D layout calculation algorithms. Completely decoupled from any GUI framework.

use crate::fenst::xplr::{BranchXplr, LeafXplr, Xplr};
use std::cell::Cell;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point
{
    pub x: f32,
    pub y: f32,
}
impl Point
{
    pub const fn New(x: f32, y: f32) -> Self { Self { x, y } }
    pub const fn Zero() -> Self { Self { x: 0.0, y: 0.0 } }
}
impl Default for Point
{
    fn default() -> Self { Self::Zero() }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimensions
{
    pub width:  f32,
    pub height: f32,
}
impl Dimensions
{
    pub const fn New(width: f32, height: f32) -> Self { Self { width, height } }
    pub const fn Zero() -> Self
    {
        Self { width:  0.0,
               height: 0.0, }
    }
}
impl Default for Dimensions
{
    fn default() -> Self { Self::Zero() }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect
{
    pub x:      f32,
    pub y:      f32,
    pub width:  f32,
    pub height: f32,
}
impl Rect
{
    pub const fn New(x: f32, y: f32, width: f32, height: f32) -> Self
    {
        Self { x,
               y,
               width,
               height }
    }
    pub const fn Zero() -> Self
    {
        Self { x:      0.0,
               y:      0.0,
               width:  0.0,
               height: 0.0, }
    }
    pub fn Right(&self) -> f32 { self.x + self.width }
    pub fn Bottom(&self) -> f32 { self.y + self.height }
    pub fn Contains(&self, px: f32, py: f32) -> bool
    {
        px >= self.x && px <= self.Right() && py >= self.y && py <= self.Bottom()
    }
}
impl Default for Rect
{
    fn default() -> Self { Self::Zero() }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color
{
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color
{
    pub const WHITE: Self = Self::Rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::Rgb(0.0, 0.0, 0.0);
    pub const TRANSPARENT: Self = Self::Rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn Rgb(r: f32, g: f32, b: f32) -> Self { Self { r, g, b, a: 1.0 } }
    pub const fn Rgba(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }
    pub const fn Hex(hex: u32) -> Self
    {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }
}
impl Default for Color
{
    fn default() -> Self { Self::WHITE }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Padding
{
    pub left:   f32,
    pub right:  f32,
    pub top:    f32,
    pub bottom: f32,
}
impl Padding
{
    pub const fn Zero() -> Self
    {
        Self { left:   0.0,
               right:  0.0,
               top:    0.0,
               bottom: 0.0, }
    }
    pub const fn All(val: f32) -> Self
    {
        Self { left:   val,
               right:  val,
               top:    val,
               bottom: val, }
    }
    pub const fn Axes(horizontal: f32, vertical: f32) -> Self
    {
        Self { left:   horizontal,
               right:  horizontal,
               top:    vertical,
               bottom: vertical, }
    }
    pub const fn New(left: f32, right: f32, top: f32, bottom: f32) -> Self
    {
        Self { left,
               right,
               top,
               bottom }
    }
    pub fn TotalHorizontal(&self) -> f32 { self.left + self.right }
    pub fn TotalVertical(&self) -> f32 { self.top + self.bottom }
}
impl Default for Padding
{
    fn default() -> Self { Self::Zero() }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDirection
{
    LeftToRight,
    TopToBottom,
}
impl Default for LayoutDirection
{
    fn default() -> Self { Self::TopToBottom }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sizing
{
    Fit,
    Fixed(f32),
    Grow,
}
impl Default for Sizing
{
    fn default() -> Self { Self::Fit }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct CaskStyle
{
    pub background_color: Option<Color>,
    pub border_color:     Option<Color>,
    pub border_width:     f32,
    pub corner_radius:    f32,
    pub text_color:       Color,
    pub font_size:        f32,
}
impl Default for CaskStyle
{
    fn default() -> Self
    {
        Self { background_color: None,
               border_color:     None,
               border_width:     0.0,
               corner_radius:    0.0,
               text_color:       Color::WHITE,
               font_size:        14.0, }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum CaskKind
{
    Window,
    Label(String),
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum CaskRenderCommand
{
    Rectangle
    {
        bounds:        Rect,
        color:         Color,
        corner_radius: f32,
    },
    Border
    {
        bounds:        Rect,
        color:         Color,
        width:         f32,
        corner_radius: f32,
    },
    Text
    {
        bounds:    Rect,
        text:      String,
        color:     Color,
        font_size: f32,
    },
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Cask
{
    pub _Id:           String,
    pub _Kind:         CaskKind,
    pub _Direction:    LayoutDirection,
    pub _Padding:      Padding,
    pub _ChildGap:     f32,
    pub _WidthSizing:  Sizing,
    pub _HeightSizing: Sizing,
    pub _Style:        CaskStyle,
    pub _Children:     Vec<Cask>,
    pub _Bounds:       Cell<Rect>,
    pub _Cursor:       Cell<Point>,
}
impl Cask
{
    pub fn NewWindow(id: impl Into<String>) -> Self
    {
        Self { _Id:           id.into(),
               _Kind:         CaskKind::Window,
               _Direction:    LayoutDirection::TopToBottom,
               _Padding:      Padding::Zero(),
               _ChildGap:     0.0,
               _WidthSizing:  Sizing::Fit,
               _HeightSizing: Sizing::Fit,
               _Style:        CaskStyle::default(),
               _Children:     Vec::new(),
               _Bounds:       Cell::new(Rect::Zero()),
               _Cursor:       Cell::new(Point::Zero()), }
    }

    pub fn NewLabel(id: impl Into<String>, text: impl Into<String>) -> Self
    {
        Self { _Id:           id.into(),
               _Kind:         CaskKind::Label(text.into()),
               _Direction:    LayoutDirection::TopToBottom,
               _Padding:      Padding::Zero(),
               _ChildGap:     0.0,
               _WidthSizing:  Sizing::Fit,
               _HeightSizing: Sizing::Fit,
               _Style:        CaskStyle::default(),
               _Children:     Vec::new(),
               _Bounds:       Cell::new(Rect::Zero()),
               _Cursor:       Cell::new(Point::Zero()), }
    }

    pub fn WithDirection(mut self, direction: LayoutDirection) -> Self
    {
        self._Direction = direction;
        self
    }

    pub fn WithPadding(mut self, padding: Padding) -> Self
    {
        self._Padding = padding;
        self
    }

    pub fn WithChildGap(mut self, gap: f32) -> Self
    {
        self._ChildGap = gap;
        self
    }

    pub fn WithWidth(mut self, sizing: Sizing) -> Self
    {
        self._WidthSizing = sizing;
        self
    }

    pub fn WithHeight(mut self, sizing: Sizing) -> Self
    {
        self._HeightSizing = sizing;
        self
    }

    pub fn WithSize(mut self, width: Sizing, height: Sizing) -> Self
    {
        self._WidthSizing = width;
        self._HeightSizing = height;
        self
    }

    pub fn WithBackground(mut self, color: Color) -> Self
    {
        self._Style.background_color = Some(color);
        self
    }

    pub fn WithBorder(mut self, color: Color, width: f32) -> Self
    {
        self._Style.border_color = Some(color);
        self._Style.border_width = width;
        self
    }

    pub fn WithCornerRadius(mut self, radius: f32) -> Self
    {
        self._Style.corner_radius = radius;
        self
    }

    pub fn WithTextColor(mut self, color: Color) -> Self
    {
        self._Style.text_color = color;
        self
    }

    pub fn WithFontSize(mut self, size: f32) -> Self
    {
        self._Style.font_size = size;
        self
    }

    pub fn WithChild(mut self, child: Cask) -> Self
    {
        self._Children.push(child);
        self
    }

    pub fn WithChildren(mut self, children: impl IntoIterator<Item = Cask>) -> Self
    {
        self._Children.extend(children);
        self
    }

    pub fn AddChild(&mut self, child: Cask) { self._Children.push(child); }

    pub fn Bounds(&self) -> Rect { self._Bounds.get() }

    pub fn SetBounds(&self, rect: Rect) { self._Bounds.set(rect); }

    pub fn Id(&self) -> &str { &self._Id }

    pub fn Kind(&self) -> &CaskKind { &self._Kind }

    pub fn Children(&self) -> &[Cask] { &self._Children }

    pub fn Style(&self) -> &CaskStyle { &self._Style }

    pub fn MeasureIntrinsic(&self) -> (f32, f32)
    {
        match &self._Kind {
            CaskKind::Label(text) => {
                let char_width = self._Style.font_size * 0.55;
                let line_height = self._Style.font_size * 1.25;
                let width = (text.len() as f32 * char_width).max(0.0);
                (width, line_height)
            }
            CaskKind::Window => {
                let b = self._Bounds.get();
                (b.width, b.height)
            }
        }
    }

    pub fn TraverseDepth<'a, F>(&'a self, lambda: F)
        where F: FnMut(&[&'a Cask], bool) -> bool
    {
        TraverseDepthRoots(&[self], lambda);
    }
}

impl Xplr for Cask
{
    fn Name(&self) -> &str { &self._Id }
    fn Path(&self) -> &str { &self._Id }
    fn IsLeaf(&self) -> bool { matches!(self._Kind, CaskKind::Label(_)) }
    fn Leaf(&self) -> Option<&dyn LeafXplr> { None }
    fn Branch(&self) -> Option<&dyn BranchXplr> { None }
}

//---------------------------------------------------------------------------------------------------------------------------------

struct CaskCursor<'a>
{
    _Children: Option<&'a [Cask]>,
    _Index:    usize,
}
impl<'a> CaskCursor<'a>
{
    fn New() -> Self
    {
        Self { _Children: None,
               _Index:    0, }
    }
    fn IsNull(&self) -> bool { self._Children.is_none() }
    fn Size(&self) -> usize
    {
        match self._Children {
            Some(children) => children.len().saturating_sub(self._Index),
            None => 0,
        }
    }
    fn CurrentChild(&self) -> Option<&'a Cask>
    {
        match self._Children {
            Some(children) if self._Index < children.len() => Some(&children[self._Index]),
            _ => None,
        }
    }
    fn Advance(&mut self) { self._Index += 1; }
}

/// Iterative depth-first traversal over multiple root `Cask` nodes without Arc or heap allocations.
///
/// - `lambda(ancestors, enter)` is called with the ancestor path slice `&[&Cask]` and `enter` flag.
/// - If `lambda` returns `false` on entry (`enter == true`), the node is treated as a leaf-node
///   (its children are not traversed and no exit call is made).
/// - If `lambda` returns `false` on exit (`enter == false`), the entire rewind process is aborted immediately.
pub fn TraverseDepthRoots<'a, F>(roots: &[&'a Cask], mut lambda: F)
    where F: FnMut(&[&'a Cask], bool) -> bool
{
    let mut tokStk: Vec<&'a Cask> = Vec::with_capacity(roots.len().max(16));
    let mut childStk: Vec<CaskCursor<'a>> = Vec::with_capacity(roots.len().max(16));
    let mut ancStk: Vec<&'a Cask> = Vec::with_capacity(32);

    for &root in roots.iter().rev() {
        tokStk.push(root);
        childStk.push(CaskCursor::New());
    }

    while let (Some(&tok), Some(childIter)) = (tokStk.last(), childStk.last_mut()) {
        let mut enterFlg = false;

        if childIter.IsNull() {
            ancStk.push(tok);
            enterFlg = true;
            let res = lambda(&ancStk, enterFlg);
            if res {
                childIter._Children = Some(tok.Children());
                childIter._Index = 0;
            } else {
                childIter._Children = Some(&[]);
                childIter._Index = 0;
            }
        }

        if childIter.Size() == 0 {
            if !enterFlg {
                let res = lambda(&ancStk, false);
                if !res {
                    return;
                }
            }
            tokStk.pop();
            childStk.pop();
            let tk = ancStk.pop();
            debug_assert_eq!(tk.map(|n| n as *const Cask), Some(tok as *const Cask));
            continue;
        }

        let nxChild = childIter.CurrentChild()
                               .expect("Valid child must exist when Size > 0");
        childIter.Advance();

        tokStk.push(nxChild);
        childStk.push(CaskCursor::New());
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Solves the 2D layout bounding boxes for a `Cask` hierarchy starting at `(origin_x, origin_y)`.
///
/// Uses two passes with iterative depth-first traversal (`TraverseDepth`):
/// - Pass 1: Postorder bottom-up measurement for `Sizing::Fit` and text intrinsics.
/// - Pass 2: Preorder top-down positioning, cursor advancement, and `Sizing::Grow` distribution.
pub fn LayoutCask(root: &Cask, origin_x: f32, origin_y: f32)
{
    // Pass 1: Bottom-up measurement for Fit sizing
    root.TraverseDepth(|ancStk, enter| {
            let node = *ancStk.last().unwrap();
            if enter {
                match &node._Kind {
                    CaskKind::Label(_) => {
                        let (w, h) = node.MeasureIntrinsic();
                        node._Bounds.set(Rect::New(0.0, 0.0, w, h));
                        return false; // Leaf node: children not traversed, no exit call
                    }
                    CaskKind::Window => {
                        let mut b = node._Bounds.get();
                        if let Sizing::Fixed(w) = node._WidthSizing {
                            b.width = w;
                        }
                        if let Sizing::Fixed(h) = node._HeightSizing {
                            b.height = h;
                        }
                        node._Bounds.set(b);
                        return true;
                    }
                }
            } else {
                // Postorder exit: all children have been measured
                if let CaskKind::Window = node._Kind {
                    let mut b = node._Bounds.get();
                    let children = node.Children();

                    if let Sizing::Fit = node._WidthSizing {
                        let content_w = match node._Direction {
                            LayoutDirection::LeftToRight => {
                                let mut sum = 0.0;
                                for c in children {
                                    sum += c._Bounds.get().width;
                                }
                                if children.len() > 1 {
                                    sum += (children.len() - 1) as f32 * node._ChildGap;
                                }
                                sum
                            }
                            LayoutDirection::TopToBottom => {
                                let mut max_w = 0.0f32;
                                for c in children {
                                    max_w = max_w.max(c._Bounds.get().width);
                                }
                                max_w
                            }
                        };
                        b.width = content_w + node._Padding.TotalHorizontal();
                    }

                    if let Sizing::Fit = node._HeightSizing {
                        let content_h = match node._Direction {
                            LayoutDirection::TopToBottom => {
                                let mut sum = 0.0;
                                for c in children {
                                    sum += c._Bounds.get().height;
                                }
                                if children.len() > 1 {
                                    sum += (children.len() - 1) as f32 * node._ChildGap;
                                }
                                sum
                            }
                            LayoutDirection::LeftToRight => {
                                let mut max_h = 0.0f32;
                                for c in children {
                                    max_h = max_h.max(c._Bounds.get().height);
                                }
                                max_h
                            }
                        };
                        b.height = content_h + node._Padding.TotalVertical();
                    }
                    node._Bounds.set(b);
                }
                true
            }
        });

    // Initialize root position
    let mut root_b = root._Bounds.get();
    root_b.x = origin_x;
    root_b.y = origin_y;
    root._Bounds.set(root_b);
    root._Cursor
        .set(Point::New(root_b.x + root._Padding.left, root_b.y + root._Padding.top));

    // Pass 2: Top-down positioning & Grow distribution
    root.TraverseDepth(|ancStk, enter| {
            let node = *ancStk.last().unwrap();
            if enter {
                if ancStk.len() >= 2 {
                    let parent = ancStk[ancStk.len() - 2];
                    let parent_b = parent._Bounds.get();
                    let cur = parent._Cursor.get();
                    let mut child_b = node._Bounds.get();

                    // Compute Grow sizing if needed
                    let parent_inner_w =
                        (parent_b.width - parent._Padding.TotalHorizontal()).max(0.0);
                    let parent_inner_h =
                        (parent_b.height - parent._Padding.TotalVertical()).max(0.0);

                    if node._WidthSizing == Sizing::Grow {
                        if parent._Direction == LayoutDirection::LeftToRight {
                            let mut grow_count = 0;
                            let mut non_grow_sum = 0.0;
                            for c in parent.Children() {
                                if c._WidthSizing == Sizing::Grow {
                                    grow_count += 1;
                                } else {
                                    non_grow_sum += c._Bounds.get().width;
                                }
                            }
                            let gaps = if parent.Children().len() > 1 {
                                (parent.Children().len() - 1) as f32 * parent._ChildGap
                            } else {
                                0.0
                            };
                            let remaining = (parent_inner_w - non_grow_sum - gaps).max(0.0);
                            child_b.width = if grow_count > 0 {
                                remaining / grow_count as f32
                            } else {
                                0.0
                            };
                        } else {
                            child_b.width = parent_inner_w;
                        }
                    }

                    if node._HeightSizing == Sizing::Grow {
                        if parent._Direction == LayoutDirection::TopToBottom {
                            let mut grow_count = 0;
                            let mut non_grow_sum = 0.0;
                            for c in parent.Children() {
                                if c._HeightSizing == Sizing::Grow {
                                    grow_count += 1;
                                } else {
                                    non_grow_sum += c._Bounds.get().height;
                                }
                            }
                            let gaps = if parent.Children().len() > 1 {
                                (parent.Children().len() - 1) as f32 * parent._ChildGap
                            } else {
                                0.0
                            };
                            let remaining = (parent_inner_h - non_grow_sum - gaps).max(0.0);
                            child_b.height = if grow_count > 0 {
                                remaining / grow_count as f32
                            } else {
                                0.0
                            };
                        } else {
                            child_b.height = parent_inner_h;
                        }
                    }

                    child_b.x = cur.x;
                    child_b.y = cur.y;
                    node._Bounds.set(child_b);

                    match parent._Direction {
                        LayoutDirection::LeftToRight => {
                            parent._Cursor
                                  .set(Point::New(cur.x + child_b.width + parent._ChildGap, cur.y));
                        }
                        LayoutDirection::TopToBottom => {
                            parent._Cursor
                                  .set(Point::New(cur.x,
                                                  cur.y + child_b.height + parent._ChildGap));
                        }
                    }
                }

                let b = node._Bounds.get();
                node._Cursor
                    .set(Point::New(b.x + node._Padding.left, b.y + node._Padding.top));

                if matches!(node._Kind, CaskKind::Label(_)) {
                    return false; // Leaf
                }
                true
            } else {
                true
            }
        });
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Solves 2D layout and emits an ordered list of `CaskRenderCommand` visual drawing commands in painter's order.
pub fn LayoutAndRenderCask(root: &Cask, origin_x: f32, origin_y: f32) -> Vec<CaskRenderCommand>
{
    LayoutCask(root, origin_x, origin_y);

    let mut commands = Vec::with_capacity(64);
    root.TraverseDepth(|ancStk, enter| {
            let node = *ancStk.last().unwrap();
            if enter {
                let b = node._Bounds.get();

                if let Some(color) = node._Style.background_color {
                    commands.push(CaskRenderCommand::Rectangle { bounds: b,
                                                                 color,
                                                                 corner_radius:
                                                                     node._Style.corner_radius });
                }

                if let Some(color) = node._Style.border_color {
                    if node._Style.border_width > 0.0 {
                        commands.push(CaskRenderCommand::Border { bounds: b,
                                                                  color,
                                                                  width: node._Style
                                                                             .border_width,
                                                                  corner_radius:
                                                                      node._Style.corner_radius });
                    }
                }

                if let CaskKind::Label(ref text) = node._Kind {
                    commands.push(CaskRenderCommand::Text { bounds:    b,
                                                            text:      text.clone(),
                                                            color:     node._Style.text_color,
                                                            font_size: node._Style.font_size, });
                    return false; // Leaf
                }
                true
            } else {
                true
            }
        });

    commands
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Semantic color scheme for Cask hierarchy rendering.
/// Provides named colors instead of scattered hex literals.
#[derive(Clone, Debug)]
pub struct CaskColorScheme
{
    pub root_bg:         Color,
    pub root_border:     Color,
    pub body_bg:         Color,
    pub body_border:     Color,
    pub module_bg:       Color,
    pub module_border:   Color,
    pub subbox_bg:       Color,
    pub subbox_border:   Color,
    pub header_text:     Color,
    pub module_text:     Color,
    pub subbox_text:     Color,
    pub leaf_green:      Color,
    pub leaf_yellow:     Color,
    pub leaf_blue:       Color,
    pub leaf_peach:      Color,
    pub leaf_lavender:   Color,
}
impl Default for CaskColorScheme
{
    fn default() -> Self
    {
        Self { root_bg:         Color::Hex(0x1E1E2E),
               root_border:     Color::Hex(0x89B4FA),
               body_bg:         Color::Hex(0x181825),
               body_border:     Color::Hex(0x313244),
               module_bg:       Color::Hex(0x24273A),
               module_border:   Color::Hex(0x585B70),
               subbox_bg:       Color::Hex(0x1E1E2E),
               subbox_border:   Color::Hex(0x45475A),
               header_text:     Color::Hex(0xCDD6F4),
               module_text:     Color::Hex(0xC6A0F6),
               subbox_text:     Color::Hex(0x8AADF4),
               leaf_green:      Color::Hex(0xA6DA95),
               leaf_yellow:     Color::Hex(0xEED49F),
               leaf_blue:       Color::Hex(0x89DCEB),
               leaf_peach:      Color::Hex(0xF5A97F),
               leaf_lavender:   Color::Hex(0xB8C0E0), }
    }
}

/// Default canvas origin offset for Cask hierarchy rendering.
pub const CASK_CANVAS_ORIGIN: (f32, f32) = (24.0, 24.0);

/// Constructs a default 3-level deep `Cask` box hierarchy for a filesystem path or explorer node.
pub fn BuildCaskHierarchyFromPath(path: &std::path::Path, max_depth: usize) -> Cask
{
    BuildCaskHierarchyFromPathStyled(path, max_depth, &CaskColorScheme::default())
}

/// Constructs a `Cask` hierarchy using a custom color scheme.
pub fn BuildCaskHierarchyFromPathStyled(path: &std::path::Path, max_depth: usize, colors: &CaskColorScheme) -> Cask
{
    let node_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("Workspace"))
        .to_string();

    let is_dir = path.is_dir();
    let icon = if is_dir { "📁" } else { "📄" };

    // Level 0: Root Container Window
    let mut root = Cask::NewWindow(format!("root_{}", node_name))
        .WithDirection(LayoutDirection::TopToBottom)
        .WithPadding(Padding::All(14.0))
        .WithChildGap(10.0)
        .WithSize(Sizing::Fixed(880.0), Sizing::Fixed(560.0))
        .WithBackground(colors.root_bg)
        .WithBorder(colors.root_border, 1.5)
        .WithCornerRadius(6.0);

    let root_header = Cask::NewLabel(
        "root_header",
        format!("📦 [Root] {} {}", icon, node_name),
    )
    .WithFontSize(15.0)
    .WithTextColor(colors.header_text);

    root.AddChild(root_header);

    if max_depth == 0 {
        return root;
    }

    let body = if is_dir {
        build_dir_body(path, max_depth, colors)
    } else {
        build_file_body(path, max_depth, colors)
    };

    root.AddChild(body);
    root
}

/// Builds the body container for directory nodes with subdirectory/file children.
fn build_dir_body(path: &std::path::Path, max_depth: usize, colors: &CaskColorScheme) -> Cask
{
    let mut body = Cask::NewWindow("root_body")
        .WithDirection(LayoutDirection::LeftToRight)
        .WithPadding(Padding::All(8.0))
        .WithChildGap(10.0)
        .WithSize(Sizing::Grow, Sizing::Grow)
        .WithBackground(colors.body_bg)
        .WithBorder(colors.body_border, 1.0)
        .WithCornerRadius(4.0);

    let mut dir_entries = Vec::with_capacity(4);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            dir_entries.push(entry.path());
            if dir_entries.len() >= 4 {
                break;
            }
        }
    }

    if dir_entries.is_empty() {
        body.AddChild(
            Cask::NewLabel("empty_label", "(empty directory)")
                .WithFontSize(12.0)
                .WithTextColor(colors.header_text),
        );
        return body;
    }

    for (idx, child_path) in dir_entries.into_iter().enumerate() {
        let child_name = child_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("entry")
            .to_string();
        let child_is_dir = child_path.is_dir();
        let child_icon = if child_is_dir { "📂" } else { "📄" };

        let mut l1_box = Cask::NewWindow(format!("l1_box_{}", idx))
            .WithDirection(LayoutDirection::TopToBottom)
            .WithPadding(Padding::All(8.0))
            .WithChildGap(6.0)
            .WithSize(Sizing::Grow, Sizing::Grow)
            .WithBackground(colors.module_bg)
            .WithBorder(colors.module_border, 1.0)
            .WithCornerRadius(4.0);

        l1_box.AddChild(
            Cask::NewLabel(
                format!("l1_header_{}", idx),
                format!("{} {}", child_icon, child_name),
            )
            .WithFontSize(13.0)
            .WithTextColor(colors.module_text),
        );

        if max_depth >= 2 {
            let mut sub_entries = Vec::with_capacity(2);
            if child_is_dir {
                if let Ok(subs) = std::fs::read_dir(&child_path) {
                    for sub in subs.flatten() {
                        sub_entries.push(sub.path());
                        if sub_entries.len() >= 2 {
                            break;
                        }
                    }
                }
            }

            if child_is_dir && !sub_entries.is_empty() {
                for (sub_idx, sub_path) in sub_entries.into_iter().enumerate() {
                    let sub_name = sub_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("item")
                        .to_string();
                    let sub_icon = if sub_path.is_dir() { "📂" } else { "📄" };

                    let mut l2_box = Cask::NewWindow(format!("l2_box_{}_{}", idx, sub_idx))
                        .WithDirection(LayoutDirection::TopToBottom)
                        .WithPadding(Padding::All(6.0))
                        .WithChildGap(4.0)
                        .WithSize(Sizing::Grow, Sizing::Grow)
                        .WithBackground(colors.subbox_bg)
                        .WithBorder(colors.subbox_border, 1.0)
                        .WithCornerRadius(3.0);

                    l2_box.AddChild(
                        Cask::NewLabel(
                            format!("l2_title_{}_{}", idx, sub_idx),
                            format!("{} {}", sub_icon, sub_name),
                        )
                        .WithFontSize(12.0)
                        .WithTextColor(colors.subbox_text),
                    );

                    if max_depth >= 3 {
                        let len_str = if let Ok(meta) = std::fs::metadata(&sub_path) {
                            format!("Size: {} B", meta.len())
                        } else {
                            "Size: 0 B".to_string()
                        };

                        l2_box.AddChild(
                            Cask::NewLabel(format!("l3_meta1_{}_{}", idx, sub_idx), len_str)
                                .WithFontSize(11.0)
                                .WithTextColor(colors.leaf_green),
                        );
                        l2_box.AddChild(
                            Cask::NewLabel(
                                format!("l3_meta2_{}_{}", idx, sub_idx),
                                if sub_path.is_dir() { "Kind: Folder" } else { "Kind: File" },
                            )
                            .WithFontSize(11.0)
                            .WithTextColor(colors.leaf_yellow),
                        );
                    }

                    l1_box.AddChild(l2_box);
                }
            } else {
                let file_size_str = if let Ok(meta) = std::fs::metadata(&child_path) {
                    format!("Size: {} B", meta.len())
                } else {
                    "Size: 0 B".to_string()
                };

                let mut l2_attr = Cask::NewWindow(format!("l2_attr_{}", idx))
                    .WithDirection(LayoutDirection::TopToBottom)
                    .WithPadding(Padding::All(6.0))
                    .WithChildGap(4.0)
                    .WithSize(Sizing::Grow, Sizing::Grow)
                    .WithBackground(colors.subbox_bg)
                    .WithBorder(colors.subbox_border, 1.0)
                    .WithCornerRadius(3.0);

                l2_attr.AddChild(
                    Cask::NewLabel(format!("l2_attr_title_{}", idx), "Attributes")
                        .WithFontSize(12.0)
                        .WithTextColor(colors.subbox_text),
                );

                if max_depth >= 3 {
                    l2_attr.AddChild(
                        Cask::NewLabel(format!("l3_attr1_{}", idx), file_size_str)
                            .WithFontSize(11.0)
                            .WithTextColor(colors.leaf_green),
                    );
                    l2_attr.AddChild(
                        Cask::NewLabel(format!("l3_attr2_{}", idx), "Status: Verified")
                            .WithFontSize(11.0)
                            .WithTextColor(colors.leaf_yellow),
                    );
                }

                let mut l2_sec = Cask::NewWindow(format!("l2_sec_{}", idx))
                    .WithDirection(LayoutDirection::TopToBottom)
                    .WithPadding(Padding::All(6.0))
                    .WithChildGap(4.0)
                    .WithSize(Sizing::Grow, Sizing::Grow)
                    .WithBackground(colors.subbox_bg)
                    .WithBorder(colors.subbox_border, 1.0)
                    .WithCornerRadius(3.0);

                l2_sec.AddChild(
                    Cask::NewLabel(format!("l2_sec_title_{}", idx), "Inspection")
                        .WithFontSize(12.0)
                        .WithTextColor(colors.subbox_text),
                );

                if max_depth >= 3 {
                    l2_sec.AddChild(
                        Cask::NewLabel(format!("l3_sec1_{}", idx), "Level: L3 Leaf")
                            .WithFontSize(11.0)
                            .WithTextColor(colors.leaf_peach),
                    );
                    l2_sec.AddChild(
                        Cask::NewLabel(format!("l3_sec2_{}", idx), "Encoding: UTF-8")
                            .WithFontSize(11.0)
                            .WithTextColor(colors.leaf_lavender),
                    );
                }

                l1_box.AddChild(l2_attr);
                l1_box.AddChild(l2_sec);
            }
        }

        body.AddChild(l1_box);
    }

    body
}

/// Builds the body container for file nodes with structured module partitions.
fn build_file_body(path: &std::path::Path, max_depth: usize, colors: &CaskColorScheme) -> Cask
{
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("data")
        .to_string();
    let file_size = if let Ok(meta) = std::fs::metadata(path) {
        format!("Size: {} B", meta.len())
    } else {
        "Size: Virtual".to_string()
    };

    let mut body = Cask::NewWindow("root_body")
        .WithDirection(LayoutDirection::LeftToRight)
        .WithPadding(Padding::All(8.0))
        .WithChildGap(10.0)
        .WithSize(Sizing::Grow, Sizing::Grow)
        .WithBackground(colors.body_bg)
        .WithBorder(colors.body_border, 1.0)
        .WithCornerRadius(4.0);

    // Helper to create a module window with common styling
    let make_module = |id: &str| -> Cask {
        Cask::NewWindow(id)
            .WithDirection(LayoutDirection::TopToBottom)
            .WithPadding(Padding::All(8.0))
            .WithChildGap(6.0)
            .WithSize(Sizing::Grow, Sizing::Grow)
            .WithBackground(colors.module_bg)
            .WithBorder(colors.module_border, 1.0)
            .WithCornerRadius(4.0)
    };

    // Helper to create a sub-box window with common styling
    let make_subbox = |id: &str| -> Cask {
        Cask::NewWindow(id)
            .WithDirection(LayoutDirection::TopToBottom)
            .WithPadding(Padding::All(6.0))
            .WithChildGap(4.0)
            .WithSize(Sizing::Grow, Sizing::Grow)
            .WithBackground(colors.subbox_bg)
            .WithBorder(colors.subbox_border, 1.0)
            .WithCornerRadius(3.0)
    };

    // Module 1: Structure & AST
    let mut m1 = make_module("mod_struct");
    m1.AddChild(
        Cask::NewLabel("m1_title", "Structure & AST")
            .WithFontSize(13.0)
            .WithTextColor(colors.module_text),
    );
    if max_depth >= 2 {
        let mut b1_1 = make_subbox("box_imports");
        b1_1.AddChild(
            Cask::NewLabel("b1_1_title", "Imports & Modules")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b1_1.AddChild(
                Cask::NewLabel("l3_b1_1", "crate::fenst::cask")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_green),
            );
            b1_1.AddChild(
                Cask::NewLabel("l3_b1_2", "std::path::Path")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_yellow),
            );
        }
        m1.AddChild(b1_1);

        let mut b1_2 = make_subbox("box_decls");
        b1_2.AddChild(
            Cask::NewLabel("b1_2_title", "Declarations")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b1_2.AddChild(
                Cask::NewLabel("l3_b2_1", "Type: Struct Cask")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_blue),
            );
            b1_2.AddChild(
                Cask::NewLabel("l3_b2_2", "Fn: TraverseDepth")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_peach),
            );
        }
        m1.AddChild(b1_2);
    }
    body.AddChild(m1);

    // Module 2: Content & Chunks
    let mut m2 = make_module("mod_content");
    m2.AddChild(
        Cask::NewLabel("m2_title", "Content & Chunks")
            .WithFontSize(13.0)
            .WithTextColor(colors.module_text),
    );
    if max_depth >= 2 {
        let mut b2_1 = make_subbox("box_block1");
        b2_1.AddChild(
            Cask::NewLabel("b2_1_title", "Block [0..100]")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b2_1.AddChild(
                Cask::NewLabel("l3_c1_1", "Length: ~100 lines")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_green),
            );
            b2_1.AddChild(
                Cask::NewLabel("l3_c1_2", "Checksum: 0x7E3A")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_yellow),
            );
        }
        m2.AddChild(b2_1);

        let mut b2_2 = make_subbox("box_block2");
        b2_2.AddChild(
            Cask::NewLabel("b2_2_title", "Block [101..250]")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b2_2.AddChild(
                Cask::NewLabel("l3_c2_1", "Tokens: 420")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_blue),
            );
            b2_2.AddChild(
                Cask::NewLabel("l3_c2_2", "Status: Parsed")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_green),
            );
        }
        m2.AddChild(b2_2);
    }
    body.AddChild(m2);

    // Module 3: Metadata & Environment
    let mut m3 = make_module("mod_meta");
    m3.AddChild(
        Cask::NewLabel("m3_title", "Metadata & Environment")
            .WithFontSize(13.0)
            .WithTextColor(colors.module_text),
    );
    if max_depth >= 2 {
        let mut b3_1 = make_subbox("box_file_attr");
        b3_1.AddChild(
            Cask::NewLabel("b3_1_title", "File Attributes")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b3_1.AddChild(
                Cask::NewLabel("l3_m1_1", file_size)
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_green),
            );
            b3_1.AddChild(
                Cask::NewLabel("l3_m1_2", format!("Format: .{}", ext.to_uppercase()))
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_yellow),
            );
        }
        m3.AddChild(b3_1);

        let mut b3_2 = make_subbox("box_env");
        b3_2.AddChild(
            Cask::NewLabel("b3_2_title", "System Context")
                .WithFontSize(12.0)
                .WithTextColor(colors.subbox_text),
        );
        if max_depth >= 3 {
            b3_2.AddChild(
                Cask::NewLabel("l3_m2_1", "Encoding: UTF-8")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_blue),
            );
            b3_2.AddChild(
                Cask::NewLabel("l3_m2_2", "Mode: Read-Only")
                    .WithFontSize(11.0)
                    .WithTextColor(colors.leaf_lavender),
            );
        }
        m3.AddChild(b3_2);
    }
    body.AddChild(m3);

    body
}
