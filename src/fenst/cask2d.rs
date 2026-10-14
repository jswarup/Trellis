//-- fenst/cask2d.rs -----------------------------------------------------------------------------------
//! 2D visual layout engine for Xplr tree hierarchies using Cask abstractions.
//!
//! Provides multi-modal 2D layout calculation (NestedBoxes, HorizontalTree, VerticalTree, IndentedList)
//! for any tree implementing the `Xplr` trait, decoupled from any GUI framework.

use crate::fenst::cask::{
    Cask, CaskRenderCommand, Color, LayoutAndRenderCask, LayoutDirection, Padding, Point,
    Rect, Sizing,
};
use crate::fenst::provider::XplrRegistry;
use crate::fenst::xplr::Xplr;

//-------------------------------------------------------------------------------------------------

/// Visual layout strategy for arranging an `Xplr` tree in 2 dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Cask2dLayoutMode {
    /// Recursive container windows where directories enclose their children (Clay/Cask style).
    #[default]
    NestedBoxes,
    /// 2D node-link tree hierarchy branching horizontally from left to right with connector lines.
    HorizontalTree,
    /// 2D node-link tree hierarchy branching vertically from top to bottom with connector lines.
    VerticalTree,
    /// Structured hierarchical outline list with branch indentation guides.
    IndentedList,
}

//-------------------------------------------------------------------------------------------------

/// Color palette for rendering 2D Cask layouts.
#[derive(Clone, Debug, PartialEq)]
pub struct Cask2dColorScheme {
    pub background_root: Color,
    pub background_branch: Color,
    pub background_leaf: Color,
    pub border_color: Color,
    pub border_highlight: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub accent_color: Color,
    pub connector_color: Color,
}

impl Default for Cask2dColorScheme {
    fn default() -> Self {
        Self::DefaultDark()
    }
}

impl Cask2dColorScheme {
    /// Dark theme color scheme (Catppuccin Mocha / dark slate inspired).
    pub fn DefaultDark() -> Self {
        Self {
            background_root: Color::Hex(0x1E1E2E),
            background_branch: Color::Hex(0x252739),
            background_leaf: Color::Hex(0x313244),
            border_color: Color::Hex(0x45475A),
            border_highlight: Color::Hex(0x89B4FA),
            text_primary: Color::Hex(0xCDD6F4),
            text_secondary: Color::Hex(0xA6ADC8),
            accent_color: Color::Hex(0x89B4FA),
            connector_color: Color::Hex(0x585B70),
        }
    }

    /// Light theme color scheme.
    pub fn DefaultLight() -> Self {
        Self {
            background_root: Color::Hex(0xEFF1F5),
            background_branch: Color::Hex(0xE6E9EF),
            background_leaf: Color::Hex(0xDCE0E8),
            border_color: Color::Hex(0xCCD0DA),
            border_highlight: Color::Hex(0x1E66F5),
            text_primary: Color::Hex(0x4C4F69),
            text_secondary: Color::Hex(0x6C6F85),
            accent_color: Color::Hex(0x1E66F5),
            connector_color: Color::Hex(0x9CA0B0),
        }
    }

    /// Returns a depth-tinted background color for containers or cards.
    pub fn color_for_depth(&self, depth: u32, is_leaf: bool) -> Color {
        if is_leaf {
            return self.background_leaf;
        }
        match depth {
            0 => self.background_root,
            1 => self.background_branch,
            2 => Color::Hex(0x2A2C40),
            3 => Color::Hex(0x32344A),
            _ => self.background_branch,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Configuration options for 2D Xplr tree layout.
#[derive(Clone, Debug, PartialEq)]
pub struct Cask2dOptions {
    pub mode: Cask2dLayoutMode,
    pub origin: Point,
    pub max_depth: u32,
    pub max_nodes: u32,
    pub node_width: f32,
    pub node_height: f32,
    pub horizontal_gap: f32,
    pub vertical_gap: f32,
    pub padding: Padding,
    pub colors: Cask2dColorScheme,
    pub show_icons: bool,
    pub show_size: bool,
}

impl Default for Cask2dOptions {
    fn default() -> Self {
        Self {
            mode: Cask2dLayoutMode::NestedBoxes,
            origin: Point::Zero(),
            max_depth: 4,
            max_nodes: 5000,
            node_width: 180.0,
            node_height: 36.0,
            horizontal_gap: 32.0,
            vertical_gap: 12.0,
            padding: Padding::All(8.0),
            colors: Cask2dColorScheme::DefaultDark(),
            show_icons: true,
            show_size: true,
        }
    }
}

impl Cask2dOptions {
    pub fn New() -> Self {
        Self::default()
    }

    pub fn WithMode(mut self, mode: Cask2dLayoutMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn WithOrigin(mut self, x: f32, y: f32) -> Self {
        self.origin = Point::New(x, y);
        self
    }

    pub fn WithMaxDepth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn WithMaxNodes(mut self, max_nodes: u32) -> Self {
        self.max_nodes = max_nodes;
        self
    }

    pub fn WithNodeSize(mut self, width: f32, height: f32) -> Self {
        self.node_width = width;
        self.node_height = height;
        self
    }

    pub fn WithGaps(mut self, horizontal: f32, vertical: f32) -> Self {
        self.horizontal_gap = horizontal;
        self.vertical_gap = vertical;
        self
    }

    pub fn WithPadding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn WithColors(mut self, colors: Cask2dColorScheme) -> Self {
        self.colors = colors;
        self
    }

    pub fn WithColorScheme(self, colors: Cask2dColorScheme) -> Self {
        self.WithColors(colors)
    }

    pub fn WithIcons(mut self, show: bool) -> Self {
        self.show_icons = show;
        self
    }

    pub fn WithSizeDisplay(mut self, show: bool) -> Self {
        self.show_size = show;
        self
    }
}

//-------------------------------------------------------------------------------------------------

/// Internal representation of an Xplr node during tree extraction.
#[derive(Clone, Debug)]
struct XplrTreeNode {
    id: String,
    name: String,
    path: String,
    is_leaf: bool,
    extension: String,
    size: u64,
    depth: u32,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// A node positioned in 2D space.
#[derive(Clone, Debug, PartialEq)]
pub struct Cask2dNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_leaf: bool,
    pub extension: String,
    pub size: u64,
    pub depth: u32,
    pub bounds: Rect,
    pub parent_index: Option<usize>,
    pub children_indices: Vec<usize>,
}

impl Cask2dNode {
    pub fn Name(&self) -> &str {
        &self.name
    }

    pub fn Path(&self) -> &str {
        &self.path
    }

    pub fn IsLeaf(&self) -> bool {
        self.is_leaf
    }

    pub fn Depth(&self) -> u32 {
        self.depth
    }

    pub fn Bounds(&self) -> Rect {
        self.bounds
    }

    pub fn Size(&self) -> u64 {
        self.size
    }

    pub fn Extension(&self) -> &str {
        &self.extension
    }

    pub fn ParentIndex(&self) -> Option<usize> {
        self.parent_index
    }

    pub fn ChildrenIndices(&self) -> &[usize] {
        &self.children_indices
    }

    /// Returns true if this node's 2D bounding box contains the specified point.
    pub fn ContainsPoint(&self, px: f32, py: f32) -> bool {
        self.bounds.Contains(px, py)
    }

    /// Returns the center point of this node's bounding box.
    pub fn Center(&self) -> Point {
        self.bounds.Center()
    }
}

//-------------------------------------------------------------------------------------------------

/// Complete solved 2D layout for an Xplr tree hierarchy.
#[derive(Clone, Debug)]
pub struct Cask2dLayout {
    pub bounds: Rect,
    pub nodes: Vec<Cask2dNode>,
    pub commands: Vec<CaskRenderCommand>,
    pub cask_tree: Option<Cask>,
}

impl Cask2dLayout {
    /// Solves the 2D layout for an `Xplr` tree starting at `root`.
    pub fn FromXplr(root: &dyn Xplr, options: &Cask2dOptions) -> Result<Self, String> {
        LayoutXplr2d(root, options)
    }

    /// Opens an `Xplr` tree from a URI through the `XplrRegistry` and solves its 2D layout.
    pub fn FromUri(uri: &str, options: &Cask2dOptions) -> Result<Self, String> {
        LayoutXplrUri(uri, options)
    }

    /// Total bounding rectangle encompassing all visual elements and padding.
    pub fn Bounds(&self) -> Rect {
        self.bounds
    }

    /// Slice of all positioned 2D nodes.
    pub fn Nodes(&self) -> &[Cask2dNode] {
        &self.nodes
    }

    /// Total number of positioned nodes.
    pub fn NodeCount(&self) -> usize {
        self.nodes.len()
    }

    /// Ordered draw commands in painter's order for rendering on a 2D canvas.
    pub fn Commands(&self) -> &[CaskRenderCommand] {
        &self.commands
    }

    /// Reference to the underlying `Cask` widget tree if constructed in `NestedBoxes` mode.
    pub fn CaskTree(&self) -> Option<&Cask> {
        self.cask_tree.as_ref()
    }

    /// Finds the topmost node containing the specified point.
    pub fn FindNodeAt(&self, point: Point) -> Option<&Cask2dNode> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.bounds.Contains(point.x, point.y))
    }

    /// Finds a node by its exact path or URI.
    pub fn FindNodeByPath(&self, path: &str) -> Option<&Cask2dNode> {
        self.nodes.iter().find(|node| node.path == path)
    }

    /// Finds the first node with a matching name.
    pub fn FindNodeByName(&self, name: &str) -> Option<&Cask2dNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    /// Maximum depth found in the laid out tree.
    pub fn MaxDepth(&self) -> u32 {
        self.nodes.iter().map(|n| n.depth).max().unwrap_or(0)
    }

    /// Count of leaf nodes.
    pub fn LeafCount(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_leaf).count()
    }

    /// Count of branch (directory/container) nodes.
    pub fn BranchCount(&self) -> usize {
        self.nodes.iter().filter(|n| !n.is_leaf).count()
    }
}

//-------------------------------------------------------------------------------------------------

/// Solves 2D layout for an `Xplr` tree using the given options.
pub fn LayoutXplr2d(root: &dyn Xplr, options: &Cask2dOptions) -> Result<Cask2dLayout, String> {
    let mut tree_nodes = Vec::new();
    CollectXplrNode(root, 0, options.max_depth, options.max_nodes, &mut tree_nodes)?;

    if tree_nodes.is_empty() {
        return Ok(Cask2dLayout {
            bounds: Rect::Zero(),
            nodes: Vec::new(),
            commands: Vec::new(),
            cask_tree: None,
        });
    }

    match options.mode {
        Cask2dLayoutMode::NestedBoxes => LayoutNestedBoxes(&tree_nodes, options),
        Cask2dLayoutMode::HorizontalTree => LayoutHorizontalTree(&tree_nodes, options),
        Cask2dLayoutMode::VerticalTree => LayoutVerticalTree(&tree_nodes, options),
        Cask2dLayoutMode::IndentedList => LayoutIndentedList(&tree_nodes, options),
    }
}

/// Opens an `Xplr` tree from a URI string (e.g. `file://`, `settings://`, `example://`) and solves its 2D layout.
pub fn LayoutXplrUri(uri: &str, options: &Cask2dOptions) -> Result<Cask2dLayout, String> {
    let registry = XplrRegistry::New();
    let (_, branch) = registry.OpenRoot(uri)?;
    LayoutXplr2d(branch.as_ref(), options)
}

/// Constructs a pure `Cask` 2D widget hierarchy from an `Xplr` tree.
pub fn XplrToCask(root: &dyn Xplr, options: &Cask2dOptions) -> Result<Cask, String> {
    let mut tree_nodes = Vec::new();
    CollectXplrNode(root, 0, options.max_depth, options.max_nodes, &mut tree_nodes)?;
    if tree_nodes.is_empty() {
        return Ok(Cask::NewWindow(root.Path().to_string()));
    }
    Ok(BuildCaskNestedBoxes(0, &tree_nodes, options))
}

//-------------------------------------------------------------------------------------------------
// Tree Ingestion Helper
//-------------------------------------------------------------------------------------------------

fn CollectXplrNode(
    node: &dyn Xplr,
    depth: u32,
    max_depth: u32,
    max_nodes: u32,
    nodes: &mut Vec<XplrTreeNode>,
) -> Result<usize, String> {
    if nodes.len() >= max_nodes as usize {
        return Err(format!("Xplr tree exceeds maximum limit of {} nodes", max_nodes));
    }
    let node_idx = nodes.len();
    let ext = node.Leaf().map(|l| l.Extension().to_string()).unwrap_or_default();
    let size = node.Leaf().map(|l| l.Size()).unwrap_or(0);

    nodes.push(XplrTreeNode {
        id: node.Path().to_string(),
        name: node.Name().to_string(),
        path: node.Path().to_string(),
        is_leaf: node.IsLeaf(),
        extension: ext,
        size,
        depth,
        parent: None,
        children: Vec::new(),
    });

    if depth < max_depth
        && !node.IsLeaf()
        && let Some(branch) = node.Branch()
        && let Ok(children) = branch.Children()
    {
        let mut sorted_indices: Vec<u32> = (0..children.Len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            let ca = &children[a];
            let cb = &children[b];
            let a_is_dir = !ca.IsLeaf();
            let b_is_dir = !cb.IsLeaf();
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                ca.Name().to_lowercase().cmp(&cb.Name().to_lowercase())
            }
        });

        for idx in sorted_indices {
            if nodes.len() >= max_nodes as usize {
                break;
            }
            let child_node = &children[idx];
            let child_idx = CollectXplrNode(
                child_node.as_ref(),
                depth + 1,
                max_depth,
                max_nodes,
                nodes,
            )?;
            nodes[child_idx].parent = Some(node_idx);
            nodes[node_idx].children.push(child_idx);
        }
    }
    Ok(node_idx)
}

//-------------------------------------------------------------------------------------------------
// Mode 1: NestedBoxes Layout (Clay / Cask Container Model)
//-------------------------------------------------------------------------------------------------

fn BuildCaskNestedBoxes(
    node_idx: usize,
    nodes: &[XplrTreeNode],
    options: &Cask2dOptions,
) -> Cask {
    let node = &nodes[node_idx];
    let depth = node.depth;
    let colors = &options.colors;

    if node.is_leaf {
        let label_text = if options.show_icons {
            let icon = icon_for_node(&node.name, true, &node.extension);
            if options.show_size && node.size > 0 {
                format!("{} {} ({})", icon, node.name, format_size(node.size))
            } else {
                format!("{} {}", icon, node.name)
            }
        } else {
            node.name.clone()
        };

        Cask::NewWindow(node.id.clone())
            .WithDirection(LayoutDirection::TopToBottom)
            .WithSize(Sizing::Fit, Sizing::Fit)
            .WithPadding(Padding::Axes(8.0, 4.0))
            .WithBackground(colors.color_for_depth(depth, true))
            .WithBorder(colors.border_color, 1.0)
            .WithCornerRadius(4.0)
            .WithChild(
                Cask::NewLabel(format!("{}_lbl", node.id), label_text)
                    .WithFontSize(12.0)
                    .WithTextColor(colors.text_primary),
            )
    } else {
        let header_text = if options.show_icons {
            let icon = icon_for_node(&node.name, false, "");
            let count_str = format!("({} items)", node.children.len());
            format!("{} {} {}", icon, node.name, count_str)
        } else {
            format!("{} ({} items)", node.name, node.children.len())
        };

        let mut window = Cask::NewWindow(node.id.clone())
            .WithDirection(LayoutDirection::TopToBottom)
            .WithSize(Sizing::Fit, Sizing::Fit)
            .WithPadding(options.padding)
            .WithChildGap(options.vertical_gap)
            .WithBackground(colors.color_for_depth(depth, false))
            .WithBorder(colors.border_color, 1.5)
            .WithCornerRadius(6.0);

        window.AddChild(
            Cask::NewLabel(format!("{}_title", node.id), header_text)
                .WithFontSize(13.0)
                .WithTextColor(colors.accent_color),
        );

        if !node.children.is_empty() {
            let mut container = Cask::NewWindow(format!("{}_cnt", node.id))
                .WithDirection(LayoutDirection::TopToBottom)
                .WithSize(Sizing::Fit, Sizing::Fit)
                .WithPadding(Padding::Axes(4.0, 4.0))
                .WithChildGap(options.vertical_gap);

            for &child_idx in &node.children {
                let child_cask = BuildCaskNestedBoxes(child_idx, nodes, options);
                container.AddChild(child_cask);
            }
            window.AddChild(container);
        }

        window
    }
}

fn LayoutNestedBoxes(
    tree_nodes: &[XplrTreeNode],
    options: &Cask2dOptions,
) -> Result<Cask2dLayout, String> {
    let root_cask = BuildCaskNestedBoxes(0, tree_nodes, options);
    let commands = LayoutAndRenderCask(&root_cask, options.origin.x, options.origin.y);
    let total_bounds = root_cask.Bounds();

    let mut id_to_bounds = std::collections::HashMap::new();
    root_cask.TraverseDepth(|anc, enter| {
        if enter {
            let node = *anc.last().unwrap();
            id_to_bounds.insert(node.Id().to_string(), node.Bounds());
        }
        true
    });

    let mut result_nodes = Vec::with_capacity(tree_nodes.len());
    for tn in tree_nodes {
        let b = id_to_bounds.get(&tn.id).copied().unwrap_or(Rect::Zero());
        result_nodes.push(Cask2dNode {
            id: tn.id.clone(),
            name: tn.name.clone(),
            path: tn.path.clone(),
            is_leaf: tn.is_leaf,
            extension: tn.extension.clone(),
            size: tn.size,
            depth: tn.depth,
            bounds: b,
            parent_index: tn.parent,
            children_indices: tn.children.clone(),
        });
    }

    Ok(Cask2dLayout {
        bounds: total_bounds,
        nodes: result_nodes,
        commands,
        cask_tree: Some(root_cask),
    })
}

//-------------------------------------------------------------------------------------------------
// Mode 2: HorizontalTree Layout (Node-Link Left-to-Right)
//-------------------------------------------------------------------------------------------------

fn LayoutHorizontalTree(
    tree_nodes: &[XplrTreeNode],
    options: &Cask2dOptions,
) -> Result<Cask2dLayout, String> {
    let n = tree_nodes.len();
    let mut subtree_heights = vec![0.0f32; n];

    // Pass 1: Bottom-up subtree height calculation
    for i in (0..n).rev() {
        let node = &tree_nodes[i];
        if node.children.is_empty() {
            subtree_heights[i] = options.node_height + options.vertical_gap;
        } else {
            let mut sum_h = 0.0f32;
            for &c in &node.children {
                sum_h += subtree_heights[c];
            }
            subtree_heights[i] = sum_h.max(options.node_height + options.vertical_gap);
        }
    }

    // Pass 2: Top-down positioning
    let mut node_bounds = vec![Rect::Zero(); n];
    position_horizontal_node(0, options.origin.y, tree_nodes, &subtree_heights, options, &mut node_bounds);

    // Pass 3: Generate visual commands and connector lines
    let mut commands = Vec::new();
    let mut total_bounds = Rect::Zero();

    // 3a. Connector lines
    for (i, tn) in tree_nodes.iter().enumerate() {
        let pb = node_bounds[i];
        for &ci in &tn.children {
            let cb = node_bounds[ci];
            let start = Point::New(pb.Right(), pb.Center().y);
            let end = Point::New(cb.x, cb.Center().y);
            let mid_x = (start.x + end.x) * 0.5;

            // Orthogonal elbow connector
            commands.push(CaskRenderCommand::Line {
                start,
                end: Point::New(mid_x, start.y),
                color: options.colors.connector_color,
                width: 1.5,
            });
            commands.push(CaskRenderCommand::Line {
                start: Point::New(mid_x, start.y),
                end: Point::New(mid_x, end.y),
                color: options.colors.connector_color,
                width: 1.5,
            });
            commands.push(CaskRenderCommand::Line {
                start: Point::New(mid_x, end.y),
                end,
                color: options.colors.connector_color,
                width: 1.5,
            });
        }
    }

    // 3b. Node cards
    for (i, tn) in tree_nodes.iter().enumerate() {
        let b = node_bounds[i];
        total_bounds = total_bounds.Union(&b);

        let bg_color = options.colors.color_for_depth(tn.depth, tn.is_leaf);
        commands.push(CaskRenderCommand::Rectangle {
            bounds: b,
            color: bg_color,
            corner_radius: 5.0,
        });
        commands.push(CaskRenderCommand::Border {
            bounds: b,
            color: options.colors.border_color,
            width: 1.0,
            corner_radius: 5.0,
        });

        let label = format_node_label(tn, options);
        let text_y = b.y + (b.height - 13.0) * 0.5;
        commands.push(CaskRenderCommand::Text {
            bounds: Rect::New(b.x + 8.0, text_y, b.width - 16.0, 16.0),
            text: label,
            color: if tn.is_leaf {
                options.colors.text_primary
            } else {
                options.colors.accent_color
            },
            font_size: 12.0,
        });
    }

    let mut result_nodes = Vec::with_capacity(n);
    for (i, tn) in tree_nodes.iter().enumerate() {
        result_nodes.push(Cask2dNode {
            id: tn.id.clone(),
            name: tn.name.clone(),
            path: tn.path.clone(),
            is_leaf: tn.is_leaf,
            extension: tn.extension.clone(),
            size: tn.size,
            depth: tn.depth,
            bounds: node_bounds[i],
            parent_index: tn.parent,
            children_indices: tn.children.clone(),
        });
    }

    Ok(Cask2dLayout {
        bounds: total_bounds,
        nodes: result_nodes,
        commands,
        cask_tree: None,
    })
}

fn position_horizontal_node(
    idx: usize,
    y_start: f32,
    nodes: &[XplrTreeNode],
    subtree_h: &[f32],
    options: &Cask2dOptions,
    bounds: &mut [Rect],
) {
    let node = &nodes[idx];
    let x = options.origin.x + node.depth as f32 * (options.node_width + options.horizontal_gap);

    if node.children.is_empty() {
        let y = y_start + (subtree_h[idx] - options.node_height) * 0.5;
        bounds[idx] = Rect::New(x, y, options.node_width, options.node_height);
    } else {
        let mut cur_y = y_start;
        for &child in &node.children {
            position_horizontal_node(child, cur_y, nodes, subtree_h, options, bounds);
            cur_y += subtree_h[child];
        }
        let first_child_y = bounds[node.children[0]].Center().y;
        let last_child_y = bounds[*node.children.last().unwrap()].Center().y;
        let parent_center_y = (first_child_y + last_child_y) * 0.5;
        let y = parent_center_y - options.node_height * 0.5;
        bounds[idx] = Rect::New(x, y, options.node_width, options.node_height);
    }
}

//-------------------------------------------------------------------------------------------------
// Mode 3: VerticalTree Layout (Node-Link Top-to-Bottom)
//-------------------------------------------------------------------------------------------------

fn LayoutVerticalTree(
    tree_nodes: &[XplrTreeNode],
    options: &Cask2dOptions,
) -> Result<Cask2dLayout, String> {
    let n = tree_nodes.len();
    let mut subtree_widths = vec![0.0f32; n];

    // Pass 1: Bottom-up subtree width calculation
    for i in (0..n).rev() {
        let node = &tree_nodes[i];
        if node.children.is_empty() {
            subtree_widths[i] = options.node_width + options.horizontal_gap;
        } else {
            let mut sum_w = 0.0f32;
            for &c in &node.children {
                sum_w += subtree_widths[c];
            }
            subtree_widths[i] = sum_w.max(options.node_width + options.horizontal_gap);
        }
    }

    // Pass 2: Top-down positioning
    let mut node_bounds = vec![Rect::Zero(); n];
    position_vertical_node(0, options.origin.x, tree_nodes, &subtree_widths, options, &mut node_bounds);

    // Pass 3: Visual commands & connectors
    let mut commands = Vec::new();
    let mut total_bounds = Rect::Zero();

    // 3a. Connector lines
    for (i, tn) in tree_nodes.iter().enumerate() {
        let pb = node_bounds[i];
        for &ci in &tn.children {
            let cb = node_bounds[ci];
            let start = Point::New(pb.Center().x, pb.Bottom());
            let end = Point::New(cb.Center().x, cb.y);
            let mid_y = (start.y + end.y) * 0.5;

            // Orthogonal elbow connector
            commands.push(CaskRenderCommand::Line {
                start,
                end: Point::New(start.x, mid_y),
                color: options.colors.connector_color,
                width: 1.5,
            });
            commands.push(CaskRenderCommand::Line {
                start: Point::New(start.x, mid_y),
                end: Point::New(end.x, mid_y),
                color: options.colors.connector_color,
                width: 1.5,
            });
            commands.push(CaskRenderCommand::Line {
                start: Point::New(end.x, mid_y),
                end,
                color: options.colors.connector_color,
                width: 1.5,
            });
        }
    }

    // 3b. Node cards
    for (i, tn) in tree_nodes.iter().enumerate() {
        let b = node_bounds[i];
        total_bounds = total_bounds.Union(&b);

        let bg_color = options.colors.color_for_depth(tn.depth, tn.is_leaf);
        commands.push(CaskRenderCommand::Rectangle {
            bounds: b,
            color: bg_color,
            corner_radius: 5.0,
        });
        commands.push(CaskRenderCommand::Border {
            bounds: b,
            color: options.colors.border_color,
            width: 1.0,
            corner_radius: 5.0,
        });

        let label = format_node_label(tn, options);
        let text_y = b.y + (b.height - 13.0) * 0.5;
        commands.push(CaskRenderCommand::Text {
            bounds: Rect::New(b.x + 8.0, text_y, b.width - 16.0, 16.0),
            text: label,
            color: if tn.is_leaf {
                options.colors.text_primary
            } else {
                options.colors.accent_color
            },
            font_size: 12.0,
        });
    }

    let mut result_nodes = Vec::with_capacity(n);
    for (i, tn) in tree_nodes.iter().enumerate() {
        result_nodes.push(Cask2dNode {
            id: tn.id.clone(),
            name: tn.name.clone(),
            path: tn.path.clone(),
            is_leaf: tn.is_leaf,
            extension: tn.extension.clone(),
            size: tn.size,
            depth: tn.depth,
            bounds: node_bounds[i],
            parent_index: tn.parent,
            children_indices: tn.children.clone(),
        });
    }

    Ok(Cask2dLayout {
        bounds: total_bounds,
        nodes: result_nodes,
        commands,
        cask_tree: None,
    })
}

fn position_vertical_node(
    idx: usize,
    x_start: f32,
    nodes: &[XplrTreeNode],
    subtree_w: &[f32],
    options: &Cask2dOptions,
    bounds: &mut [Rect],
) {
    let node = &nodes[idx];
    let y = options.origin.y + node.depth as f32 * (options.node_height + options.vertical_gap);

    if node.children.is_empty() {
        let x = x_start + (subtree_w[idx] - options.node_width) * 0.5;
        bounds[idx] = Rect::New(x, y, options.node_width, options.node_height);
    } else {
        let mut cur_x = x_start;
        for &child in &node.children {
            position_vertical_node(child, cur_x, nodes, subtree_w, options, bounds);
            cur_x += subtree_w[child];
        }
        let first_child_x = bounds[node.children[0]].Center().x;
        let last_child_x = bounds[*node.children.last().unwrap()].Center().x;
        let parent_center_x = (first_child_x + last_child_x) * 0.5;
        let x = parent_center_x - options.node_width * 0.5;
        bounds[idx] = Rect::New(x, y, options.node_width, options.node_height);
    }
}

//-------------------------------------------------------------------------------------------------
// Mode 4: IndentedList Layout (Hierarchical Outline with Guide Lines)
//-------------------------------------------------------------------------------------------------

fn LayoutIndentedList(
    tree_nodes: &[XplrTreeNode],
    options: &Cask2dOptions,
) -> Result<Cask2dLayout, String> {
    let n = tree_nodes.len();
    let mut commands = Vec::new();
    let mut node_bounds = vec![Rect::Zero(); n];
    let mut total_bounds = Rect::Zero();

    let row_height = options.node_height;
    let row_gap = options.vertical_gap;
    let indent_step = options.horizontal_gap;

    for (i, tn) in tree_nodes.iter().enumerate() {
        let x = options.origin.x + tn.depth as f32 * indent_step;
        let y = options.origin.y + i as f32 * (row_height + row_gap);
        let b = Rect::New(x, y, options.node_width, row_height);
        node_bounds[i] = b;
        total_bounds = total_bounds.Union(&b);

        let bg_color = options.colors.color_for_depth(tn.depth, tn.is_leaf);
        commands.push(CaskRenderCommand::Rectangle {
            bounds: b,
            color: bg_color,
            corner_radius: 4.0,
        });
        commands.push(CaskRenderCommand::Border {
            bounds: b,
            color: options.colors.border_color,
            width: 1.0,
            corner_radius: 4.0,
        });

        let label = format_node_label(tn, options);
        let text_y = b.y + (b.height - 13.0) * 0.5;
        commands.push(CaskRenderCommand::Text {
            bounds: Rect::New(b.x + 8.0, text_y, b.width - 16.0, 16.0),
            text: label,
            color: if tn.is_leaf {
                options.colors.text_primary
            } else {
                options.colors.accent_color
            },
            font_size: 12.0,
        });
    }

    // Branch guide lines connecting parents to their children
    for (i, tn) in tree_nodes.iter().enumerate() {
        if !tn.children.is_empty() {
            let pb = node_bounds[i];
            let last_child_idx = *tn.children.last().unwrap();
            let last_cb = node_bounds[last_child_idx];
            let guide_x = pb.x + 12.0;

            // Vertical guide line
            commands.push(CaskRenderCommand::Line {
                start: Point::New(guide_x, pb.Bottom()),
                end: Point::New(guide_x, last_cb.Center().y),
                color: options.colors.connector_color,
                width: 1.0,
            });

            // Horizontal stubs to each child
            for &ci in &tn.children {
                let cb = node_bounds[ci];
                commands.push(CaskRenderCommand::Line {
                    start: Point::New(guide_x, cb.Center().y),
                    end: Point::New(cb.x, cb.Center().y),
                    color: options.colors.connector_color,
                    width: 1.0,
                });
            }
        }
    }

    let mut result_nodes = Vec::with_capacity(n);
    for (i, tn) in tree_nodes.iter().enumerate() {
        result_nodes.push(Cask2dNode {
            id: tn.id.clone(),
            name: tn.name.clone(),
            path: tn.path.clone(),
            is_leaf: tn.is_leaf,
            extension: tn.extension.clone(),
            size: tn.size,
            depth: tn.depth,
            bounds: node_bounds[i],
            parent_index: tn.parent,
            children_indices: tn.children.clone(),
        });
    }

    Ok(Cask2dLayout {
        bounds: total_bounds,
        nodes: result_nodes,
        commands,
        cask_tree: None,
    })
}

//-------------------------------------------------------------------------------------------------
// Helpers: Labels, Icons, and Size Formatting
//-------------------------------------------------------------------------------------------------

fn format_node_label(node: &XplrTreeNode, options: &Cask2dOptions) -> String {
    if options.show_icons {
        let icon = icon_for_node(&node.name, node.is_leaf, &node.extension);
        if node.is_leaf && options.show_size && node.size > 0 {
            format!("{} {} ({})", icon, node.name, format_size(node.size))
        } else if !node.is_leaf && !node.children.is_empty() {
            format!("{} {} ({})", icon, node.name, node.children.len())
        } else {
            format!("{} {}", icon, node.name)
        }
    } else if node.is_leaf && options.show_size && node.size > 0 {
        format!("{} ({})", node.name, format_size(node.size))
    } else {
        node.name.clone()
    }
}

pub fn icon_for_node(name: &str, is_leaf: bool, ext: &str) -> &'static str {
    if !is_leaf {
        if name.contains(':') || name == "file://" {
            return "💾";
        }
        return "📁";
    }
    match ext.to_lowercase().as_str() {
        "rs" => "🦀",
        "toml" | "json" | "yaml" | "yml" | "cfg" => "⚙",
        "md" | "txt" | "log" => "📝",
        "vcd" => "📈",
        "obj" | "pts" => "🧊",
        "theme" => "🎨",
        "py" => "🐍",
        _ => "📄",
    }
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
