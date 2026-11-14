use crate::fenst::cask::Cask;
use crate::fenst::cask_scene::{CaskScene, ViewBox};
use crate::fenst::{BranchXplr, FsBranch, FsLeaf, LeafXplr, Xplr, XplrRegistry};
use crate::{jeeves_assert, jeeves_assert_eq, jeeves_test};

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, FilesystemExplorer, XplrExample, |ctx| {
    let branch = FsBranch::New("src".to_string());
    let children = branch.Children().expect("src should be readable");
    jeeves_assert!(ctx, !children.IsEmpty());
    let leaf = FsLeaf::New("Cargo.toml".to_string());
    jeeves_assert!(ctx, leaf.IsLeaf());
    jeeves_assert_eq!(ctx, leaf.Extension(), "toml");
    let chunk = leaf.ReadChunk(0, 64)
                    .expect("Cargo.toml should be readable");
    jeeves_assert!(ctx, chunk.Content().contains("[package]"));
    jeeves_assert!(ctx, !chunk.IsEof());
    return Ok(Box::new(branch));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, SketchExample, XplrExample, |ctx| {
    use crate::fenst::{SketchDocument, SketchPoint, SketchXplr, Xplr};

    let mut doc = SketchDocument::New();
    let _ = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(120.0, 0.0));
    let _ = doc.AddLine(SketchPoint::New(120.0, 0.0), SketchPoint::New(120.0, 80.0));
    let _ = doc.AddLine(SketchPoint::New(120.0, 80.0), SketchPoint::New(0.0, 80.0));
    let _ = doc.AddLine(SketchPoint::New(0.0, 80.0), SketchPoint::New(0.0, 0.0));

    let _ = doc.AddCircle(SketchPoint::New(60.0, 40.0), 25.0);
    let _ = doc.AddArc(SketchPoint::New(60.0, 40.0),
                       15.0,
                       0.0,
                       std::f64::consts::PI);
    let _ = doc.AddEllipse(SketchPoint::New(60.0, 40.0), 35.0, 18.0, 0.0);
    jeeves_assert!(ctx, doc.EntityCount() >= 7);

    let xplr = SketchXplr::FromDocument(&doc, "MechanicalSketch", "example://Fenst/SketchExample");
    let boxed: Box<dyn Xplr + Send> = Box::new(xplr);
    return Ok(boxed);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, ProviderRegistry, |ctx| {
    let registry = XplrRegistry::New();
    let (scheme, root) = registry.OpenRoot("file://src")
                                 .expect("file provider should open src");
    jeeves_assert_eq!(ctx, scheme, "file");
    jeeves_assert_eq!(ctx, root.Name(), "src");
    jeeves_assert!(ctx, root.ChildCount().expect("src count") > 0);
    jeeves_assert!(ctx, registry.OpenRoot("missing://root").is_err());
});

jeeves_test!(Fenst, FilesystemRootsXplrHierarchy, |ctx| {
    let registry = XplrRegistry::New();
    let (scheme, root) = registry.OpenRoot("file://")
                                 .expect("file provider should open root");
    jeeves_assert_eq!(ctx, scheme, "file");
    jeeves_assert_eq!(ctx, root.Name(), "Filesystem");
    jeeves_assert_eq!(ctx, root.Path(), "file://");
    let children = root.Children().expect("filesystem children");
    jeeves_assert!(ctx, !children.IsEmpty());
    #[cfg(target_os = "windows")]
    {
        let mut has_c = false;
        children.Traverse(|drive| {
                    if drive.Name() == "C:" {
                        has_c = true;
                        let drive_branch = drive.Branch().expect("drive branch");
                        let c_children = drive_branch.Children();
                        jeeves_assert!(ctx, c_children.is_ok());
                    }
                });
        jeeves_assert!(ctx, has_c);
    }
});

jeeves_test!(Fenst, SettingsXplrHierarchy, |ctx| {
    let registry = XplrRegistry::New();
    let (scheme, root) = registry.OpenRoot("settings://")
                                 .expect("settings provider should open root");
    jeeves_assert_eq!(ctx, scheme, "settings");
    jeeves_assert_eq!(ctx, root.Name(), "Settings");
    jeeves_assert_eq!(ctx, root.ChildCount().expect("category count"), 3);
    let categories = root.Children().expect("categories");
    jeeves_assert_eq!(ctx, categories.Len(), 3);
    jeeves_assert_eq!(ctx, categories[0].Name(), "Theme");
    jeeves_assert_eq!(ctx, categories[1].Name(), "Layout");
    jeeves_assert_eq!(ctx, categories[2].Name(), "System");

    let theme_branch = categories[0].Branch().expect("theme branch");
    jeeves_assert_eq!(ctx, theme_branch.ChildCount().expect("theme count"), 5);
    let themes = theme_branch.Children().expect("themes");
    jeeves_assert_eq!(ctx, themes[0].Name(), "WindowsDark");
    let leaf = themes[0].Leaf().expect("dark leaf");
    jeeves_assert_eq!(ctx, leaf.Extension(), "theme");
    let chunk = leaf.ReadChunk(0, 100).expect("read dark theme");
    jeeves_assert!(ctx, chunk.Content().contains("Windows Dark"));

    let system_branch = categories[2].Branch().expect("system branch");
    jeeves_assert_eq!(ctx, system_branch.ChildCount().expect("system count"), 3);
    let sys_children = system_branch.Children().expect("sys children");
    let mut has_sim_workers = false;
    sys_children.Traverse(|c| {
                    if c.Name() == "SimulationWorkers" {
                        has_sim_workers = true;
                    }
                });
    jeeves_assert!(ctx, has_sim_workers);
});

jeeves_test!(Fenst, ExamplesXplrHierarchy, |ctx| {
    let registry = XplrRegistry::New();
    let (scheme, root) = registry.OpenRoot("example://")
                                 .expect("example provider should open root");
    jeeves_assert_eq!(ctx, scheme, "example");
    jeeves_assert_eq!(ctx, root.Name(), "Examples");
    jeeves_assert!(ctx, root.ChildCount().expect("suite count") > 0);
    let suites = root.Children().expect("suites");
    let mut found_fenst = false;
    suites.Traverse(|s| {
              if s.Name() == "Fenst" {
                  found_fenst = true;
                  let examples = s.Branch()
                                  .expect("fenst suite branch")
                                  .Children()
                                  .expect("fenst examples");
                  jeeves_assert!(ctx, examples.Len() > 0);
                  let first = &examples[0];
                  jeeves_assert!(ctx, first.Branch().is_some());
                  let run_res = first.Branch().unwrap().Children();
                  jeeves_assert!(ctx, run_res.is_ok());
              }
          });
    jeeves_assert!(ctx, found_fenst);
});

//---------------------------------------------------------------------------------------------------------------------------------

struct MockLeaf(&'static str);
impl Xplr for MockLeaf
{
    fn Name(&self) -> &str { self.0 }
    fn Path(&self) -> &str { self.0 }
    fn IsLeaf(&self) -> bool { true }
    fn Leaf(&self) -> Option<&dyn LeafXplr> { return Some(self); }
}

impl LeafXplr for MockLeaf
{
    fn Size(&self) -> u64 { return self.0.len() as u64; }

    fn Extension(&self) -> &str { return "txt"; }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<crate::fenst::StreamChunk, String>
    {
        let start = usize::try_from(offset).unwrap_or(usize::MAX)
                                           .min(self.0.len());
        let end = start.saturating_add(length as usize).min(self.0.len());
        return Ok(crate::fenst::StreamChunk::New(self.0.into(),
                                                 offset,
                                                 self.Size(),
                                                 self.0[start..end].into()));
    }
}

struct MockBranch
{
    name:        &'static str,
    children_fn: fn() -> crate::silo::Buff<Box<dyn Xplr>>,
}
impl Xplr for MockBranch
{
    fn Name(&self) -> &str { self.name }
    fn Path(&self) -> &str { self.name }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}
impl BranchXplr for MockBranch
{
    fn Children(&self) -> Result<crate::silo::Buff<Box<dyn Xplr>>, String>
    {
        Ok((self.children_fn)())
    }
    fn ChildCount(&self) -> Result<u32, String> { Ok((self.children_fn)().Len()) }
}

fn BuildMockTree() -> MockBranch
{
    fn make_dir_a() -> crate::silo::Buff<Box<dyn Xplr>>
    {
        let mut s = crate::silo::Stash::New();
        s.Push(Box::new(MockLeaf("FileA1")) as Box<dyn Xplr>);
        s.Push(Box::new(MockLeaf("FileA2")) as Box<dyn Xplr>);
        s.IntoBuff()
    }
    fn make_dir_b() -> crate::silo::Buff<Box<dyn Xplr>>
    {
        let mut s = crate::silo::Stash::New();
        s.Push(Box::new(MockLeaf("FileB1")) as Box<dyn Xplr>);
        s.IntoBuff()
    }
    fn make_root() -> crate::silo::Buff<Box<dyn Xplr>>
    {
        let mut s = crate::silo::Stash::New();
        s.Push(Box::new(MockBranch { name:        "DirA",
                                     children_fn: make_dir_a, }) as Box<dyn Xplr>);
        s.Push(Box::new(MockBranch { name:        "DirB",
                                     children_fn: make_dir_b, }) as Box<dyn Xplr>);
        s.Push(Box::new(MockLeaf("FileRoot")) as Box<dyn Xplr>);
        s.IntoBuff()
    }
    MockBranch { name:        "Root",
                 children_fn: make_root, }
}

jeeves_test!(Fenst, TraverseDepthFullOrder, XplrExample, |ctx| {
    let root = BuildMockTree();
    let mut events: Vec<(String, bool)> = Vec::new();
    let mut ancestor_paths: Vec<String> = Vec::new();

    root.TraverseDepth(|ancestors, enter| {
            let curr = ancestors.last().expect("must have current node");
            events.push((curr.Name().to_string(), enter));

            let path = ancestors.iter()
                                .map(|a| a.Name())
                                .collect::<Vec<_>>()
                                .join("/");
            if enter {
                ancestor_paths.push(path);
            }
            true
        });

    // Check complete preorder enter / postorder exit event sequence
    let expected_events = [
        ( "Root".to_string(), true),
        ( "DirA".to_string(), true),
        ( "FileA1".to_string(), true),   // Leaf: enter only, no exit call
        ( "FileA2".to_string(), true),   // Leaf: enter only, no exit call
        ( "DirA".to_string(), false),  // Branch: exit call
        ( "DirB".to_string(), true),
        ( "FileB1".to_string(), true),   // Leaf: enter only, no exit call
        ( "DirB".to_string(), false),  // Branch: exit call
        ( "FileRoot".to_string(), true), // Leaf: enter only, no exit call
        ( "Root".to_string(), false),  // Branch: exit call
    ];
    jeeves_assert_eq!(ctx, events.len(), expected_events.len());
    for (i, expected) in expected_events.iter().enumerate() {
        jeeves_assert_eq!(ctx, &events[i], expected);
    }

    // Check ancestor paths
    let expected_paths = ["Root",
                          "Root/DirA",
                          "Root/DirA/FileA1",
                          "Root/DirA/FileA2",
                          "Root/DirB",
                          "Root/DirB/FileB1",
                          "Root/FileRoot"];
    jeeves_assert_eq!(ctx, ancestor_paths.len(), expected_paths.len());
    for (i, expected) in expected_paths.iter().enumerate() {
        jeeves_assert_eq!(ctx, &ancestor_paths[i], expected);
    }
    return Ok(Box::new(root));
});

jeeves_test!(Fenst, XplrExampleRegistrationAndFailure, |ctx| {
    use crate::cove::{TestContext, TestKind};
    use crate::fenst::example::{XplrExample, XplrExampleResult};

    let examples = XplrExample::Registered();
    let mut found = false;
    examples.Traverse(|example| {
                if example.suite == "Fenst" && example.name == "TraverseDepthFullOrder" {
                    found = true;
                    let root = example.Run().unwrap();
                    jeeves_assert_eq!(ctx, root.Name(), "Root");
                    jeeves_assert_eq!(ctx, root.Branch().unwrap().ChildCount().unwrap(), 3);
                }
            });
    jeeves_assert!(ctx, found);
    let mut standard = false;
    inventory::iter::<crate::cove::TestCase>.into_iter()
                                            .for_each(|case| {
                                                if case.suite == "Fenst"
                                                   && case.name == "TraverseDepthFullOrder"
                                                {
                                                    standard = case.kind == TestKind::Example;
                                                }
                                            });
    jeeves_assert!(ctx, standard);
    examples.USeg().Traverse(|index| {
                       if index > 0 {
                           let a = examples[index - 1];
                           let b = examples[index];
                           jeeves_assert!(ctx, (a.suite, a.name) <= (b.suite, b.name));
                       }
                   });
    fn FailedAssertion(ctx: &mut TestContext) -> XplrExampleResult
    {
        jeeves_assert!(ctx, false);
        return Ok(Box::new(BuildMockTree()));
    }
    fn FailedResult(_ctx: &mut TestContext) -> XplrExampleResult
    {
        return Err("Example error".into());
    }
    fn Panicked(_ctx: &mut TestContext) -> XplrExampleResult
    {
        panic!("Example panic");
    }
    let case = XplrExample { suite: "Fixture",
                             name:  "Failure",
                             func:  FailedAssertion, };
    jeeves_assert_eq!(ctx, case.Run().err().unwrap(), "1 assertion(s) failed");
    let case = XplrExample { func: FailedResult,
                             ..case };
    jeeves_assert_eq!(ctx, case.Run().err().unwrap(), "Example error");
    let case = XplrExample { func: Panicked,
                             ..case };
    jeeves_assert_eq!(ctx,
                      case.Run().err().unwrap(),
                      "Example panicked: Example panic");
});

jeeves_test!(Fenst, TraverseDepthPruneOnEntry, |ctx| {
    let root = BuildMockTree();
    let mut visited = Vec::new();

    root.TraverseDepth(|ancestors, enter| {
            let curr = ancestors.last().unwrap();
            visited.push((curr.Name().to_string(), enter));
            // Prune DirA on entry: should treat DirA as a leaf-node (no children, no exit call)
            if curr.Name() == "DirA" && enter {
                return false;
            }
            true
        });

    let expected = [("Root".to_string(), true),
                    ("DirA".to_string(), true), // Pruned on entry: treated as leaf, no exit call!
                    ("DirB".to_string(), true),
                    ("FileB1".to_string(), true),
                    ("DirB".to_string(), false),
                    ("FileRoot".to_string(), true),
                    ("Root".to_string(), false)];
    jeeves_assert_eq!(ctx, visited.len(), expected.len());
    for (i, exp) in expected.iter().enumerate() {
        jeeves_assert_eq!(ctx, &visited[i], exp);
    }
});

jeeves_test!(Fenst, TraverseDepthAbortOnExit, |ctx| {
    let root = BuildMockTree();
    let mut visited = Vec::new();

    root.TraverseDepth(|ancestors, enter| {
            let curr = ancestors.last().unwrap();
            visited.push((curr.Name().to_string(), enter));
            // Abort traversal on exit of DirA
            if curr.Name() == "DirA" && !enter {
                return false;
            }
            true
        });

    // Traversal should abort immediately upon DirA exit returning false.
    // DirB, FileRoot, and Root exit must never be visited.
    let expected = [
        ( "Root".to_string(), true),
        ( "DirA".to_string(), true),
        ( "FileA1".to_string(), true),
        ( "FileA2".to_string(), true),
        ( "DirA".to_string(), false),  // Aborted here
    ];
    jeeves_assert_eq!(ctx, visited.len(), expected.len());
    for (i, exp) in expected.iter().enumerate() {
        jeeves_assert_eq!(ctx, &visited[i], exp);
    }
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskLabelIntrinsicMeasurement, |ctx| {
    use crate::fenst::cask::{Cask, LayoutAndRenderCask, Rect};

    let label = Cask::NewLabel("msg", "Hello").WithFontSize(10.0);
    let (w, h) = label.MeasureIntrinsic();
    // 5 chars * 10.0 * 0.55 = 27.5, line height = 10.0 * 1.25 = 12.5
    jeeves_assert_eq!(ctx, w, 27.5);
    jeeves_assert_eq!(ctx, h, 12.5);

    let commands = LayoutAndRenderCask(&label, 50.0, 100.0);
    jeeves_assert_eq!(ctx, commands.len(), 1);
    jeeves_assert_eq!(ctx, label.Bounds(), Rect::New(50.0, 100.0, 27.5, 12.5));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskHorizontalLayoutWithPaddingAndGap, |ctx| {
    use crate::fenst::cask::{Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing};

    let root =
        Cask::NewWindow("row").WithDirection(LayoutDirection::LeftToRight)
                              .WithPadding(Padding::Axes(10.0, 5.0))
                              .WithChildGap(8.0)
                              .WithChild(Cask::NewWindow("c1").WithSize(Sizing::Fixed(100.0),
                                                                        Sizing::Fixed(50.0)))
                              .WithChild(Cask::NewWindow("c2").WithSize(Sizing::Fixed(200.0),
                                                                        Sizing::Fixed(60.0)));

    let _ = LayoutAndRenderCask(&root, 0.0, 0.0);

    // Parent fit: width = 100 + 200 + 8 (gap) + 20 (padding) = 328.0
    // Parent fit: height = max(50, 60) + 10 (padding) = 70.0
    jeeves_assert_eq!(ctx, root.Bounds(), Rect::New(0.0, 0.0, 328.0, 70.0));

    let children = root.Children();
    // c1 bounds: x = 10.0, y = 5.0, w = 100.0, h = 50.0
    jeeves_assert_eq!(ctx, children[0].Bounds(), Rect::New(10.0, 5.0, 100.0, 50.0));
    // c2 bounds: x = 10.0 + 100.0 + 8.0 = 118.0, y = 5.0, w = 200.0, h = 60.0
    jeeves_assert_eq!(ctx,
                      children[1].Bounds(),
                      Rect::New(118.0, 5.0, 200.0, 60.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskVerticalLayoutWithPaddingAndGap, |ctx| {
    use crate::fenst::cask::{Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing};

    let  	root = Cask::NewWindow( "column")
        .WithDirection( LayoutDirection::TopToBottom)
        .WithPadding( Padding::All( 12.0))
        .WithChildGap( 4.0)
        .WithChild(
            Cask::NewWindow( "item1")
                .WithSize( Sizing::Fixed( 150.0), Sizing::Fixed( 30.0))
        )
        .WithChild(
            Cask::NewWindow( "item2")
                .WithSize( Sizing::Fixed( 180.0), Sizing::Fixed( 40.0))
        );

    let _ = LayoutAndRenderCask(&root, 100.0, 50.0);

    // Parent fit: width = max(150, 180) + 24 (padding) = 204.0
    // Parent fit: height = 30 + 40 + 4 (gap) + 24 (padding) = 98.0
    jeeves_assert_eq!(ctx, root.Bounds(), Rect::New(100.0, 50.0, 204.0, 98.0));

    let children = root.Children();
    // item1: x = 112.0, y = 62.0, w = 150.0, h = 30.0
    jeeves_assert_eq!(ctx,
                      children[0].Bounds(),
                      Rect::New(112.0, 62.0, 150.0, 30.0));
    // item2: x = 112.0, y = 62.0 + 30.0 + 4.0 = 96.0, w = 180.0, h = 40.0
    jeeves_assert_eq!(ctx,
                      children[1].Bounds(),
                      Rect::New(112.0, 96.0, 180.0, 40.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskGrowDistribution, |ctx| {
    use crate::fenst::cask::{Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing};

    let  	root = Cask::NewWindow( "grow_container")
        .WithDirection( LayoutDirection::LeftToRight)
        .WithPadding( Padding::All( 10.0))
        .WithChildGap( 10.0)
        .WithSize( Sizing::Fixed( 500.0), Sizing::Fixed( 300.0))
        .WithChild(
            Cask::NewWindow( "fixed_child")
                .WithSize( Sizing::Fixed( 100.0), Sizing::Fixed( 50.0))
        )
        .WithChild(
            Cask::NewWindow( "grow_child_1")
                .WithSize( Sizing::Grow, Sizing::Grow)
        )
        .WithChild(
            Cask::NewWindow( "grow_child_2")
                .WithSize( Sizing::Grow, Sizing::Fixed( 60.0))
        );

    let _ = LayoutAndRenderCask(&root, 0.0, 0.0);

    // Inner width = 500 - 20 = 480.
    // Non-grow width = 100. Gaps = 2 * 10 = 20.
    // Remaining = 480 - 100 - 20 = 360.
    // Each of the 2 grow children gets 360 / 2 = 180.0 width!
    // Inner height = 300 - 20 = 280.
    // grow_child_1 cross grow height = 280.0.

    let children = root.Children();
    // fixed_child: x = 10.0, y = 10.0, w = 100.0, h = 50.0
    jeeves_assert_eq!(ctx,
                      children[0].Bounds(),
                      Rect::New(10.0, 10.0, 100.0, 50.0));
    // grow_child_1: x = 10 + 100 + 10 = 120.0, y = 10.0, w = 180.0, h = 280.0
    jeeves_assert_eq!(ctx,
                      children[1].Bounds(),
                      Rect::New(120.0, 10.0, 180.0, 280.0));
    // grow_child_2: x = 120 + 180 + 10 = 310.0, y = 10.0, w = 180.0, h = 60.0
    jeeves_assert_eq!(ctx,
                      children[2].Bounds(),
                      Rect::New(310.0, 10.0, 180.0, 60.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskNestedClayWindowRenderCommands, |ctx| {
    use crate::fenst::cask::{
        Cask, CaskRenderCommand, Color, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing,
    };

    // Multi-level Clay-style window layout:
    // Window (TopToBottom)
    //   ├── TitleBar (LeftToRight)
    //   │     ├── Title Label
    //   │     └── Close Button Label
    //   └── Body (LeftToRight)
    //         ├── Sidebar (TopToBottom)
    //         │     └── Nav Item Label
    //         └── Content (TopToBottom)
    //               └── Content Label

    let  	root = Cask::NewWindow( "Window")
        .WithDirection( LayoutDirection::TopToBottom)
        .WithSize( Sizing::Fixed( 400.0), Sizing::Fixed( 300.0))
        .WithPadding( Padding::All( 4.0))
        .WithChildGap( 4.0)
        .WithBackground( Color::Hex( 0x1E1E2E))
        .WithBorder( Color::Hex( 0x45475A), 1.0)
        .WithChild(
            Cask::NewWindow( "TitleBar")
                .WithDirection( LayoutDirection::LeftToRight)
                .WithSize( Sizing::Grow, Sizing::Fixed( 32.0))
                .WithPadding( Padding::Axes( 8.0, 4.0))
                .WithBackground( Color::Hex( 0x313244))
                .WithChild(
                    Cask::NewLabel( "TitleText", "Trellis Studio")
                        .WithTextColor( Color::WHITE)
                        .WithFontSize( 14.0)
                )
        )
        .WithChild(
            Cask::NewWindow( "Body")
                .WithDirection( LayoutDirection::LeftToRight)
                .WithSize( Sizing::Grow, Sizing::Grow)
                .WithChildGap( 4.0)
                .WithChild(
                    Cask::NewWindow( "Sidebar")
                        .WithSize( Sizing::Fixed( 100.0), Sizing::Grow)
                        .WithBackground( Color::Hex( 0x181825))
                        .WithPadding( Padding::All( 6.0))
                        .WithChild(
                            Cask::NewLabel( "NavFiles", "Files")
                                .WithTextColor( Color::Hex( 0xCDD6F4))
                                .WithFontSize( 12.0)
                        )
                )
                .WithChild(
                    Cask::NewWindow( "Content")
                        .WithSize( Sizing::Grow, Sizing::Grow)
                        .WithBackground( Color::Hex( 0x11111B))
                        .WithPadding( Padding::All( 8.0))
                        .WithChild(
                            Cask::NewLabel( "ContentText", "Editor Workspace")
                                .WithTextColor( Color::Hex( 0xA6ADC8))
                                .WithFontSize( 13.0)
                        )
                )
        );

    let commands = LayoutAndRenderCask(&root, 10.0, 20.0);

    // Verify root window bounds
    jeeves_assert_eq!(ctx, root.Bounds(), Rect::New(10.0, 20.0, 400.0, 300.0));

    // Verify TitleBar: x = 14.0, y = 24.0, w = 400 - 8 = 392.0, h = 32.0
    let title_bar = &root.Children()[0];
    jeeves_assert_eq!(ctx, title_bar.Bounds(), Rect::New(14.0, 24.0, 392.0, 32.0));

    // Verify Body: x = 14.0, y = 24.0 + 32.0 + 4.0 = 60.0, w = 392.0, h = 300 - 8 - 32 - 4 = 256.0
    let body = &root.Children()[1];
    jeeves_assert_eq!(ctx, body.Bounds(), Rect::New(14.0, 60.0, 392.0, 256.0));

    // Verify Sidebar: x = 14.0, y = 60.0, w = 100.0, h = 256.0
    let sidebar = &body.Children()[0];
    jeeves_assert_eq!(ctx, sidebar.Bounds(), Rect::New(14.0, 60.0, 100.0, 256.0));

    // Verify Content: x = 14.0 + 100.0 + 4.0 = 118.0, y = 60.0, w = 392 - 100 - 4 = 288.0, h = 256.0
    let content = &body.Children()[1];
    jeeves_assert_eq!(ctx, content.Bounds(), Rect::New(118.0, 60.0, 288.0, 256.0));

    // Check emitted commands count and types
    jeeves_assert!(ctx, !commands.is_empty());

    // Verify first command is root Window background
    match &commands[0] {
        CaskRenderCommand::Rectangle { bounds, color, .. } => {
            jeeves_assert_eq!(ctx, *bounds, Rect::New(10.0, 20.0, 400.0, 300.0));
            jeeves_assert_eq!(ctx, *color, Color::Hex(0x1E1E2E));
        }
        _ => panic!("Expected Rectangle command for root background"),
    }

    // Verify second command is root Window border
    match &commands[1] {
        CaskRenderCommand::Border { bounds,
                                    color,
                                    width,
                                    .. } => {
            jeeves_assert_eq!(ctx, *bounds, Rect::New(10.0, 20.0, 400.0, 300.0));
            jeeves_assert_eq!(ctx, *color, Color::Hex(0x45475A));
            jeeves_assert_eq!(ctx, *width, 1.0);
        }
        _ => panic!("Expected Border command for root border"),
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskSceneLayoutInvariants, |ctx| {
    use crate::fenst::cask::Cask;
    use crate::fenst::cask_scene::CaskScene;
    use crate::silo::USeg;
    let mut branch = Cask::NewWindow("deep leaf");
    USeg::FromLen( 6).Traverse( |i| branch = Cask::NewWindow( format!( "level {i}")).WithChild( std::mem::replace( &mut branch, Cask::NewWindow( "unused"))));
    let mut root = Cask::NewWindow("root").WithChild(branch);
    USeg::FromLen( 20).Traverse( |i| root = std::mem::replace( &mut root, Cask::NewWindow( "unused")).WithChild( Cask::NewWindow( format!( "sibling {i}"))));
    let mut scene = CaskScene::FromRoot(&root).unwrap();
    scene.Layout(&std::sync::atomic::AtomicBool::new(false), |name| {
             [name.len() as f32 * 7.0, 20.0]
         })
         .unwrap();
    let nodes = scene.Nodes();
    jeeves_assert_eq!(ctx, scene.MaxDepth(), 8);
    jeeves_assert_eq!(ctx, nodes.Size(), 28);
    let mut zVariation = false;
    nodes.USeg().Traverse(|i| {
                    let node = &nodes[i];
                    let p = node.Origin();
                    let size = node.Size();
                    if node.IsLeaf() {
                        jeeves_assert_eq!(ctx, size[1], size[2]);
                    }
                    if node.Parent() != u32::MAX {
                        let parent = &nodes[node.Parent()];
                        USeg::FromLen(3).Traverse(|axis| {
                                            let a = axis as usize;
                                            jeeves_assert!(ctx, p[a] >= parent.Origin()[a]);
                                            jeeves_assert!(ctx,
                                                           p[a] + size[a]
                                                           <= parent.Origin()[a]
                                                              + parent.Size()[a]
                                                              + 0.001);
                                        });
                    }
                    USeg::WithLen(i + 1, nodes.Size() - i - 1).Traverse(|j| {
                        let other = &nodes[j];
                        if node.Depth() != other.Depth() {
                            return;
                        }
                        let mut disjoint = false;
                        USeg::FromLen(3).Traverse(|axis| {
                                            let a = axis as usize;
                                            disjoint |= p[a] + size[a] <= other.Origin()[a]
                                                        || other.Origin()[a] + other.Size()[a]
                                                           <= p[a];
                                        });
                        jeeves_assert!(ctx, disjoint, "Same-level casks must be disjoint");
                        if node.Parent() == other.Parent() {
                            zVariation |= p[2] != other.Origin()[2];
                        }
                    });
                });
    jeeves_assert!(ctx, zVariation, "Packing must use Z as well as Y");
    let bounds = scene.Bounds();
    scene.Layout(&std::sync::atomic::AtomicBool::new(false), |name| {
             [name.len() as f32 * 7.0, 20.0]
         })
         .unwrap();
    jeeves_assert_eq!(ctx, bounds, scene.Bounds());
    let labels = Cask::NewWindow("labels").WithChild(Cask::NewLabel("first", "First"))
                                          .WithChild(Cask::NewLabel("second", "Second"));
    let labelled = CaskScene::FromRoot(&labels).unwrap();
    let nodes = labelled.Nodes();
    jeeves_assert_eq!(ctx, nodes[0].Name(), "First\nSecond");
});

jeeves_test!(Fenst, CaskSceneReadsAllDepths, |ctx| {
    use crate::fenst::cask_scene::CaskScene;
    use crate::silo::{IArr, USeg};
    use std::sync::atomic::AtomicBool;
    // Unique, test-owned directory. Drop removes only this fixture, including on panic.
    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture
    {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    let path = std::env::temp_dir().join(format!(
        "trellis-cask-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    let fixture = Fixture(path);
    let mut leaf = fixture.0.clone();
    USeg::FromLen(7).Traverse(|i| {
                        leaf.push(format!("d{i}"));
                        std::fs::create_dir(&leaf).unwrap();
                    });
    std::fs::write(leaf.join("leaf.txt"), "leaf").unwrap();
    USeg::FromLen(9).Traverse(|i| {
                        std::fs::write(fixture.0.join(format!("sibling-{i}.txt")), "").unwrap()
                    });
    let scene = CaskScene::Read(&fixture.0, &AtomicBool::new(false)).unwrap();
    jeeves_assert_eq!(ctx, scene.MaxDepth(), 9);
    jeeves_assert_eq!(ctx, scene.Nodes().Size(), 18);
    let mut deepest = 0;
    scene.Nodes()
         .Traverse(|node| deepest = deepest.max(node.Depth()));
    jeeves_assert_eq!(ctx, deepest, scene.MaxDepth());
    jeeves_assert_eq!(ctx, scene.Nodes()[0].Height(), 8);
    scene.Nodes().Traverse(|node| {
                     if node.IsLeaf() {
                         jeeves_assert_eq!(ctx, node.Height(), 0);
                     }
                 });
    jeeves_assert!(ctx,
                   CaskScene::Read(&fixture.0, &AtomicBool::new(true)).is_err());
    jeeves_assert!(ctx,
                   CaskScene::Read(&fixture.0.join("missing"), &AtomicBool::new(false)).is_err());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CaskSceneCancellationAndBudget, |ctx| {
    use crate::fenst::cask::Cask;
    use crate::fenst::cask_scene::{CaskScene, ICaskHierarchy};
    use crate::silo::USeg;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Hierarchy<'a>
    {
        _Cancelled: &'a AtomicBool,
        _Visited:   Cell<u32>,
        _CancelAt:  u32,
        _Width:     u32,
    }

    impl ICaskHierarchy for Hierarchy<'_>
    {
        type Node = u32;

        fn Label(&self, node: u32) -> String
        {
            self._Visited.set(self._Visited.get() + 1);
            if node == self._CancelAt {
                self._Cancelled.store(true, Ordering::Release);
            }
            "node".into()
        }

        fn SpanChildren(&self, node: u32, mut visit: impl FnMut(u32) -> bool)
        {
            if node == 0 {
                USeg::FromLen(self._Width).Span(|index| visit(index + 1));
            }
        }
    }

    let cancelled = AtomicBool::new(false);
    let source = Hierarchy { _Cancelled: &cancelled,
                             _Visited:   Cell::new(0),
                             _CancelAt:  2,
                             _Width:     100, };
    let result = CaskScene::FromHierarchy(&source, 0, &cancelled);
    jeeves_assert_eq!(ctx, result.unwrap_err(), "Loading cancelled.");
    jeeves_assert_eq!(ctx, source._Visited.get(), 3);
    cancelled.store(false, Ordering::Release);
    let source = Hierarchy { _Cancelled: &cancelled,
                             _Visited:   Cell::new(0),
                             _CancelAt:  u32::MAX,
                             _Width:     100_000, };
    let result = CaskScene::FromHierarchy(&source, 0, &cancelled);
    jeeves_assert!(ctx, result.unwrap_err().contains("100,000"));
    jeeves_assert_eq!(ctx, source._Visited.get(), 100_000);
    let atLimit = Hierarchy { _Cancelled: &cancelled,
                              _Visited:   Cell::new(0),
                              _CancelAt:  u32::MAX,
                              _Width:     99_999, };
    let scene = CaskScene::FromHierarchy(&atLimit, 0, &cancelled).unwrap();
    jeeves_assert_eq!(ctx, scene.Nodes().Size(), 100_000);
    drop(scene);
    let root = Cask::NewWindow("root").WithChild(Cask::NewWindow("first"))
                                      .WithChild(Cask::NewWindow("second"));
    let mut scene = CaskScene::FromRoot(&root).unwrap();
    let mut measured = 0;
    let result = scene.Layout(&cancelled, |_| {
                          measured += 1;
                          cancelled.store(true, Ordering::Release);
                          [20.0; 2]
                      });
    jeeves_assert_eq!(ctx, result.unwrap_err(), "Loading cancelled.");
    jeeves_assert_eq!(ctx, measured, 1);
    let result = scene.Layout(&cancelled,
                              |_| panic!("Cancelled layout must not measure text"));
    jeeves_assert_eq!(ctx, result.unwrap_err(), "Loading cancelled.");
    cancelled.store(false, Ordering::Release);
    scene.Layout(&cancelled, |_| [20.0; 2]).unwrap();
    jeeves_assert!(ctx, scene.Bounds().1.iter().all(|size| *size > 0.0));
    cancelled.store(true, Ordering::Release);
    let result = CaskScene::FromHierarchy(&source, 0, &cancelled);
    jeeves_assert_eq!(ctx, result.unwrap_err(), "Loading cancelled.");
    jeeves_assert_eq!(ctx, source._Visited.get(), 100_000);
});

jeeves_test!(Fenst, CaskSceneViewBoxAndUnfurling, |ctx| {
    use std::sync::atomic::AtomicBool;

    // 1. Verify ViewBox depth is 100x max view dimension.
    let vb = ViewBox::New([800.0, 600.0], [0.0; 3], 1.0);
    jeeves_assert_eq!(ctx, vb.ViewSize(), [800.0, 600.0]);
    jeeves_assert_eq!(ctx, vb.MaxViewDimension(), 800.0);
    jeeves_assert_eq!(ctx, vb.Depth(), 800.0 * 100.0);
    jeeves_assert_eq!(ctx, vb.ScaleViewToWorld(), 1.0);
    jeeves_assert_eq!(ctx, vb.ScaleWorldToView(), 1.0);
    jeeves_assert_eq!(ctx,
                      vb.GpuMemoryThreshold(),
                      ViewBox::DEFAULT_GPU_MEMORY_THRESHOLD_BYTES);

    // 2. Verify scale applied to the world box of objects.
    let zoomed = ViewBox::New([800.0, 600.0], [0.0; 3], 0.5);
    jeeves_assert_eq!(ctx, zoomed.ScaleWorldToView(), 2.0);
    let (_scaled_orig, scaled_sz) = zoomed.ScaledWorldBox([10.0, 20.0, 0.0], [100.0, 50.0, 30.0]);
    jeeves_assert_eq!(ctx, scaled_sz, [200.0, 100.0, 60.0]);
    jeeves_assert_eq!(ctx, zoomed.ResolutionPixels([100.0, 50.0, 30.0]), 200.0);

    // 3. Verify sub-pixel resolution test (< few pixels, default threshold 3.0).
    jeeves_assert!(ctx, !vb.ShouldUnfurl([0.0; 3], [1.0, 1.0, 1.0]));
    jeeves_assert_eq!(ctx, vb.ResolutionPixels([1.0, 1.0, 1.0]), 1.0);
    jeeves_assert!(ctx, vb.ShouldUnfurl([0.0; 3], [10.0, 10.0, 10.0]));

    // 4. Verify out-of-viewbox culling.
    jeeves_assert!(ctx,
                   !vb.ShouldUnfurl([5000.0, 5000.0, 0.0], [50.0, 50.0, 50.0]));

    // 5. Test CaskScene unfurling on a hierarchical tree.
    let tiny_leaf = Cask::NewWindow("great_grandchild");
    let tiny_parent = Cask::NewWindow("tiny").WithChild(tiny_leaf);
    let child1 = Cask::NewWindow("child1").WithChild(Cask::NewWindow("grandchild1a"))
                                          .WithChild(tiny_parent);
    let child2 = Cask::NewWindow("child2_offscreen").WithChild(Cask::NewWindow("grandchild2a"));
    let root = Cask::NewWindow("root").WithChild(child1).WithChild(child2);

    let mut scene = CaskScene::FromRoot(&root).unwrap();
    scene.Layout(&AtomicBool::new(false), |name| {
             if name == "tiny" {
                 [0.5, 0.5]
             } else if name == "child2_offscreen" {
                 [100.0, 20.0]
             } else {
                 [80.0, 20.0]
             }
         })
         .unwrap();

    let full_count = scene.Nodes().Size();
    jeeves_assert!(ctx, full_count >= 6);

    // Sub-pixel pruning: calibrate threshold between child and root
    let root_size = scene.Nodes()[0].Size()[0].max(scene.Nodes()[0].Size()[1]);
    let child_size = scene.Nodes()[1].Size()[0].max(scene.Nodes()[1].Size()[1]);
    let threshold = (child_size + root_size) * 0.5;
    let zoomed_out_vb = ViewBox::New( [800.0, 600.0], scene.Nodes()[0].Origin(), 1.0)
        .WithMinPixelThreshold( threshold );
    let unfurled_subpixel = scene.Unfurl(&zoomed_out_vb, &AtomicBool::new(false))
                                 .unwrap();
    jeeves_assert!(ctx,
                   unfurled_subpixel.Nodes().Size() < full_count,
                   "Sub-pixel entities must not unfurl");

    // Off-screen pruning: viewbox tightly bounds child1 so child2 is offscreen
    let child1_node = &scene.Nodes()[1];
    let child1_orig = child1_node.Origin();
    let offscreen_vb = ViewBox::New([20.0, 20.0], child1_orig, 1.0).WithMinPixelThreshold(0.0);
    let unfurled_offscreen = scene.Unfurl(&offscreen_vb, &AtomicBool::new(false))
                                  .unwrap();
    jeeves_assert!(ctx,
                   unfurled_offscreen.Nodes().Size() < full_count,
                   "Offscreen entities must not unfurl");
    jeeves_assert_eq!(ctx, unfurled_offscreen.Bounds(), scene.Bounds());
    jeeves_assert!(ctx, unfurled_offscreen.MaxDepth() <= scene.MaxDepth());
    jeeves_assert!(ctx, !unfurled_offscreen.Nodes()[0].IsLeaf());

    // 6. GPU memory threshold capping:
    // With budget for only 3 nodes, the tree unfurls only down to the budget limit
    let budget_bytes = 3 * ViewBox::GPU_BYTES_PER_NODE;
    let gpu_capped_vb = ViewBox::New( [800.0, 600.0], scene.Nodes()[0].Origin(), 1.0)
        .WithMinPixelThreshold( 0.0)
        .WithGpuMemoryThreshold( budget_bytes );
    jeeves_assert_eq!(ctx, gpu_capped_vb.MaxNodesForGpuBudget(), 3);
    let unfurled_gpu = scene.Unfurl(&gpu_capped_vb, &AtomicBool::new(false))
                            .unwrap();
    jeeves_assert!(ctx,
                   unfurled_gpu.Nodes().Size() <= 3,
                   "Unfurled node count must respect GPU memory threshold");
    jeeves_assert!(ctx, unfurled_gpu.Nodes().Size() < full_count);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, Cask2dLayoutModesOnMockXplrTree, |ctx| {
    use crate::fenst::cask2d::{Cask2dLayoutMode, Cask2dOptions, LayoutXplr2d};

    let root = BuildMockTree();

    // 1. NestedBoxes layout
    let mut opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::NestedBoxes)
                                       .WithOrigin(50.0, 50.0);
    let layout_boxes = LayoutXplr2d(&root, &opts).expect("nested boxes layout");
    jeeves_assert_eq!(ctx, layout_boxes.NodeCount(), 7); // Root, DirA (FileA1, FileA2), DirB (FileB1), FileRoot
    jeeves_assert!(ctx, layout_boxes.Bounds().width > 0.0);
    jeeves_assert!(ctx, layout_boxes.Bounds().height > 0.0);
    jeeves_assert!(ctx, layout_boxes.CaskTree().is_some());
    jeeves_assert!(ctx, !layout_boxes.Commands().is_empty());

    // Test queries
    let root_node = layout_boxes.FindNodeByName("Root").expect("find Root");
    jeeves_assert_eq!(ctx, root_node.depth, 0);
    jeeves_assert!(ctx, !root_node.is_leaf);
    jeeves_assert_eq!(ctx, root_node.children_indices.len(), 3);

    let file_a1 = layout_boxes.FindNodeByName("FileA1").expect("find FileA1");
    jeeves_assert_eq!(ctx, file_a1.depth, 2);
    jeeves_assert!(ctx, file_a1.is_leaf);

    // Test point collision
    let hit = layout_boxes.FindNodeAt(file_a1.Center());
    jeeves_assert!(ctx, hit.is_some());
    jeeves_assert_eq!(ctx, hit.unwrap().name, "FileA1");

    // 2. HorizontalTree layout
    opts.mode = Cask2dLayoutMode::HorizontalTree;
    let layout_htree = LayoutXplr2d(&root, &opts).expect("horizontal tree layout");
    jeeves_assert_eq!(ctx, layout_htree.NodeCount(), 7);
    jeeves_assert!(ctx, layout_htree.Bounds().width > 0.0);
    let r_node = layout_htree.FindNodeByName("Root").unwrap();
    let da_node = layout_htree.FindNodeByName("DirA").unwrap();
    let fa1_node = layout_htree.FindNodeByName("FileA1").unwrap();
    jeeves_assert!(ctx, da_node.bounds.x > r_node.bounds.x);
    jeeves_assert!(ctx, fa1_node.bounds.x > da_node.bounds.x);

    // 3. VerticalTree layout
    opts.mode = Cask2dLayoutMode::VerticalTree;
    let layout_vtree = LayoutXplr2d(&root, &opts).expect("vertical tree layout");
    jeeves_assert_eq!(ctx, layout_vtree.NodeCount(), 7);
    let r_v = layout_vtree.FindNodeByName("Root").unwrap();
    let da_v = layout_vtree.FindNodeByName("DirA").unwrap();
    let fa1_v = layout_vtree.FindNodeByName("FileA1").unwrap();
    jeeves_assert!(ctx, da_v.bounds.y > r_v.bounds.y);
    jeeves_assert!(ctx, fa1_v.bounds.y > da_v.bounds.y);

    // 4. IndentedList layout
    opts.mode = Cask2dLayoutMode::IndentedList;
    let layout_list = LayoutXplr2d(&root, &opts).expect("indented list layout");
    jeeves_assert_eq!(ctx, layout_list.NodeCount(), 7);
    jeeves_assert!(ctx, !layout_list.Commands().is_empty());
});

jeeves_test!(Fenst, Cask2dFromSettingsAndExamplesUri, |ctx| {
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};

    let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::HorizontalTree)
                                   .WithMaxDepth(3);

    // 1. settings:// layout
    let settings_layout = Cask2dLayout::FromUri("settings://", &opts).expect("layout settings");
    jeeves_assert!(ctx, settings_layout.NodeCount() >= 4);
    jeeves_assert!(ctx, settings_layout.FindNodeByName("Theme").is_some());
    jeeves_assert!(ctx, settings_layout.FindNodeByName("WindowsDark").is_some());

    // 2. example:// layout
    let examples_layout = Cask2dLayout::FromUri("example://", &opts).expect("layout examples");
    jeeves_assert!(ctx, examples_layout.NodeCount() >= 2);
    jeeves_assert!(ctx, examples_layout.FindNodeByName("Examples").is_some());
});

jeeves_test!(Fenst, Cask2dMaxDepthPruning, |ctx| {
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dOptions};

    let root = BuildMockTree();

    // Depth 0: only Root
    let opts0 = Cask2dOptions::New().WithMaxDepth(0);
    let layout0 = Cask2dLayout::FromXplr(&root, &opts0).unwrap();
    jeeves_assert_eq!(ctx, layout0.NodeCount(), 1);

    // Depth 1: Root + direct children (DirA, DirB, FileRoot) = 4
    let opts1 = Cask2dOptions::New().WithMaxDepth(1);
    let layout1 = Cask2dLayout::FromXplr(&root, &opts1).unwrap();
    jeeves_assert_eq!(ctx, layout1.NodeCount(), 4);

    // Depth 2: All 7 nodes
    let opts2 = Cask2dOptions::New().WithMaxDepth(2);
    let layout2 = Cask2dLayout::FromXplr(&root, &opts2).unwrap();
    jeeves_assert_eq!(ctx, layout2.NodeCount(), 7);
});

//-------------------------------------------------------------------------------------------------
// Classical 2D Circuit Schematic & 3D Circuit CaskScene via generic CircuitXplr
jeeves_test!(
             Fenst,
             CircuitSchematicClassicalLayoutAndManhattanRouting,
             |ctx| {
                 use crate::fenst::cask::CaskRenderCommand;
                 use crate::fenst::cask_scene::CaskScene;
                 use crate::fenst::cask2d::{Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};
                 use crate::fenst::xplr::{
                     CircuitXplr, PortDescriptor, PortDirection, WireDescriptor, Xplr,
                 };
                 use std::sync::atomic::AtomicBool;

                 struct MockGate
    {
                     name: &'static str,
                     kind: &'static str,
                 }
                 impl Xplr for MockGate
                 {
        fn Name(&self) -> &str { self.name }
        fn Path(&self) -> &str { self.name }
        fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
    }
                 impl CircuitXplr for MockGate
                 {
        fn ComponentType(&self) -> &str { self.kind }
        fn Ports(&self) -> Vec<PortDescriptor>
        {
            vec![PortDescriptor::New("in0", PortDirection::In),
                 PortDescriptor::New("in1", PortDirection::In),
                 PortDescriptor::New("out", PortDirection::Out),]
        }
        fn Wires(&self) -> Vec<WireDescriptor> { Vec::new() }
        fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>> { Vec::new() }
    }

                 struct MockCircuit;
                 impl Xplr for MockCircuit
                 {
        fn Name(&self) -> &str { "ALU" }
        fn Path(&self) -> &str { "ALU" }
        fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
    }
                 impl CircuitXplr for MockCircuit
                 {
        fn ComponentType(&self) -> &str { "Module" }
        fn Ports(&self) -> Vec<PortDescriptor>
        {
            vec![PortDescriptor::New("a", PortDirection::In),
                 PortDescriptor::New("b", PortDirection::In),
                 PortDescriptor::New("res", PortDirection::Out),]
        }
        fn Wires(&self) -> Vec<WireDescriptor>
        {
            vec![WireDescriptor::New("ALU", "a", "G1", "in0"),
                 WireDescriptor::New("ALU", "b", "G1", "in1"),
                 WireDescriptor::New("G1", "out", "G2", "in0"),
                 WireDescriptor::New("ALU", "b", "G2", "in1"),
                 WireDescriptor::New("G2", "out", "ALU", "res"),]
        }
        fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
        {
            vec![Box::new(MockGate { name: "G1",
                                     kind: "AND", }),
                 Box::new(MockGate { name: "G2",
                                     kind: "OR", }),]
        }
    }

                 let circuit = MockCircuit;

                 // 1. Classical 2D Schematic
                 let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::CircuitSchematic);
                 let layout = Cask2dLayout::FromCircuit(&circuit, &opts);

                 // Root + 2 gates = 3 nodes
                 jeeves_assert_eq!(ctx, layout.NodeCount(), 3);
                 let g1 = layout.Nodes().iter().find(|n| n.Name() == "G1").unwrap();
                 let g2 = layout.Nodes().iter().find(|n| n.Name() == "G2").unwrap();
                 // G1 drives G2, so G1 must be staged to the left of G2
                 jeeves_assert!(ctx, g1.Bounds().x < g2.Bounds().x);

                 // Verify orthogonal Manhattan wire routing commands
                 let mut line_count = 0;
                 for cmd in layout.Commands() {
                     if matches!(cmd, CaskRenderCommand::Line { .. }) {
                         line_count += 1;
                     }
                 }
                 jeeves_assert!(ctx, line_count >= 5);

                 // 2. 3D Circuit Scene
                 let scene = CaskScene::FromCircuit(&circuit, &AtomicBool::new(false)).unwrap();
                 jeeves_assert!(ctx, scene.Nodes().Size() >= 3);
             }
);

//-------------------------------------------------------------------------------------------------
jeeves_test!(Fenst, Cask2dCircuitSchematicFromExampleUri, |ctx| {
    use crate::fenst::cask::CaskRenderCommand;
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};

    let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::CircuitSchematic);
    let layout =
        Cask2dLayout::FromUri("example://Rube/FullAdderExample", &opts).expect("layout example");

    // FullAdder root + 3 submodules = 4 nodes
    jeeves_assert_eq!(ctx, layout.NodeCount(), 4);
    jeeves_assert_eq!(ctx, layout.Nodes()[0].Name(), "FullAdder");

    // Classical topological column staging: HA1 (left) < HA2 (mid) < Or (right)
    let ha1 = layout.Nodes()
                    .iter()
                    .find(|n| n.Name().contains("HA1"))
                    .unwrap();
    let ha2 = layout.Nodes()
                    .iter()
                    .find(|n| n.Name().contains("HA2"))
                    .unwrap();
    let or_gate = layout.Nodes()
                        .iter()
                        .find(|n| n.Name().contains("Or"))
                        .unwrap();

    jeeves_assert!(ctx, ha1.Bounds().x < ha2.Bounds().x);
    jeeves_assert!(ctx, ha2.Bounds().x < or_gate.Bounds().x);

    // Verify wire lines and soldered junction dots
    let mut line_count = 0;
    let mut dot_count = 0;
    for cmd in layout.Commands() {
        match cmd {
            CaskRenderCommand::Line { .. } => line_count += 1,
            CaskRenderCommand::Rectangle { corner_radius,
                                           bounds,
                                           .. } if *corner_radius == 3.0 && bounds.width == 6.0 => {
                dot_count += 1;
            }
            _ => {}
        }
    }
    jeeves_assert!(ctx, line_count >= 8);
    jeeves_assert!(ctx, dot_count >= 1);

    // Karst Top-Level System Schematic
    let karst_system = Cask2dLayout::FromUri("example://Karst/System", &opts).expect("layout karst top-level system");
    // Top-level Karst root + 8 hosts + 1 KarstModule + 8 MemChans = 18 nodes
    jeeves_assert_eq!(ctx,
                      karst_system.NodeCount(),
                      18,
                      "Karst top-level schematic must have 8 hosts, 1 KarstModule, and 8 MemChans");
    jeeves_assert_eq!(ctx, karst_system.Nodes()[0].Name(), "Karst");
    let km = karst_system.Nodes()
                         .iter()
                         .find(|n| n.Name() == "KarstModule")
                         .expect("KarstModule must exist");

    // All 8 Hosts must be on the LEFT (x < KarstModule.x)
    for i in 0..8 {
        let name = format!("Host {}", i);
        let host = karst_system.Nodes().iter().find(|n| n.Name() == name);
        jeeves_assert!(ctx, host.is_some(), "Karst must contain Host");
        jeeves_assert!(ctx,
                       host.unwrap().Bounds().x < km.Bounds().x,
                       "Host must be on the left of KarstModule");
    }

    // All 8 DDR5 Memory Channels must be on the RIGHT (x > KarstModule.x)
    for i in 0..8 {
        let name = format!("MemChan {}", i);
        let mc = karst_system.Nodes().iter().find(|n| n.Name() == name);
        jeeves_assert!(ctx, mc.is_some(), "Karst must contain MemChan");
        jeeves_assert!(ctx,
                       km.Bounds().x < mc.unwrap().Bounds().x,
                       "MemChan must be on the right of KarstModule");
    }

    // Check Karst system level wire lines and pin triangles
    let mut system_lines = 0;
    let mut system_triangles = 0;
    for cmd in karst_system.Commands() {
        match cmd {
            CaskRenderCommand::Line { .. } => system_lines += 1,
            CaskRenderCommand::Triangle { .. } => system_triangles += 1,
            _ => {}
        }
    }
    jeeves_assert!(ctx,
                   system_lines >= 10,
                   "Karst system must route Karst-level interconnect wires");
    jeeves_assert!(ctx,
                   system_triangles >= 10,
                   "InPorts and OutPorts must display triangles/arrowheads");

    // Karst Die2 Submodule Schematic under KarstModule
    let karst_die2 = Cask2dLayout::FromUri("example://Karst/System/KarstModule/Die2Circuit", &opts).expect("layout karst die2");
    // Die2 root + 1 die leaf component (NoC 1) = 2 nodes
    jeeves_assert_eq!(ctx,
                      karst_die2.NodeCount(),
                      2,
                      "Die2Circuit must show strictly its 1 submodule (NoC 1)");
    jeeves_assert_eq!(ctx, karst_die2.Nodes()[0].Name(), "Die2Circuit");
    let mut die2_lines = 0;
    let mut die2_triangles = 0;
    for cmd in karst_die2.Commands() {
        match cmd {
            CaskRenderCommand::Line { .. } => die2_lines += 1,
            CaskRenderCommand::Triangle { .. } => die2_triangles += 1,
            _ => {}
        }
    }
    jeeves_assert!(ctx,
                   die2_lines >= 10,
                   "Die2Circuit must have intra-die wire lines");
    jeeves_assert!(ctx,
                   die2_triangles >= 10,
                   "Die2Circuit InPorts and OutPorts must have triangles");

    // Karst Die1 Submodule Schematic under KarstModule
    let karst_die1 = Cask2dLayout::FromUri("example://Karst/System/KarstModule/Die1Circuit", &opts).expect("layout karst die1");
    jeeves_assert_eq!(ctx,
                      karst_die1.NodeCount(),
                      2,
                      "Die1Circuit must show strictly its 1 submodule (NoC 0)");
    jeeves_assert_eq!(ctx, karst_die1.Nodes()[0].Name(), "Die1Circuit");

    // Karst Fabric Module Submodule layout
    let karst_fabric = Cask2dLayout::FromUri("example://Karst/System/KarstModule", &opts).expect("layout karst fabric");
    let mut fabric_lines = 0;
    for cmd in karst_fabric.Commands() {
        if matches!(cmd, CaskRenderCommand::Line { .. }) {
            fabric_lines += 1;
        }
    }
    jeeves_assert!(ctx, fabric_lines >= 2, "Karst fabric must have wire lines");

    // Suite Folders: Karst and Rube open as folders listing their XplrExamples
    let karst_folder = Cask2dLayout::FromUri("example://Karst", &opts).expect("layout karst suite");
    jeeves_assert_eq!(ctx, karst_folder.Nodes()[0].Name(), "Karst");
    jeeves_assert!(ctx, karst_folder.FindNodeByName("System").is_some());

    let rube_suite = Cask2dLayout::FromUri("example://Rube", &opts).expect("layout rube suite");
    jeeves_assert_eq!(ctx, rube_suite.Nodes()[0].Name(), "Rube");
    jeeves_assert!(ctx, rube_suite.FindNodeByName("FullAdderExample").is_some());
    jeeves_assert!(ctx, rube_suite.FindNodeByName("Adder8Example").is_some());
    jeeves_assert!(ctx,
                   rube_suite.FindNodeByName("DaddaMultiplier4x4").is_some());

    // Rube Submodule Navigation: HalfAdder HA1 (FullAdder submodule)
    let ha1_circuit = Cask2dLayout::FromUri("example://Rube/FullAdderExample/HA1", &opts).expect("layout ha1 submodule");
    jeeves_assert_eq!(ctx, ha1_circuit.Nodes()[0].Name(), "HA1");
    // HA1 root + 2 submodules (Xor, And) = 3 nodes
    jeeves_assert_eq!(ctx, ha1_circuit.NodeCount(), 3);
    let xor_mod = ha1_circuit.Nodes().iter().find(|n| n.Name() == "Xor");
    let and_mod = ha1_circuit.Nodes().iter().find(|n| n.Name() == "And");
    jeeves_assert!(ctx, xor_mod.is_some(), "HA1 must contain Xor submodule");
    jeeves_assert!(ctx, and_mod.is_some(), "HA1 must contain And submodule");

    // Rube Leaf Gate Navigation: XOR gate (HA1 submodule)
    let xor_circuit = Cask2dLayout::FromUri("example://Rube/FullAdderExample/HA1/Xor", &opts).expect("layout xor gate");
    jeeves_assert_eq!(ctx, xor_circuit.Nodes()[0].Name(), "Xor");
    jeeves_assert_eq!(ctx,
                      xor_circuit.NodeCount(),
                      2,
                      "Leaf XOR gate has canvas and component block");

    // Rube Adder8 Hierarchical Example
    let adder8 =
        Cask2dLayout::FromUri("example://Rube/Adder8Example", &opts).expect("layout adder8");
    jeeves_assert_eq!(ctx, adder8.Nodes()[0].Name(), "Adder8");
    // Adder8 root + 8 full adders Bit0..Bit7 = 9 nodes
    jeeves_assert_eq!(ctx, adder8.NodeCount(), 9, "Adder8 must have 8 submodules");

    // Rube Adder8 Submodule Bit0
    let bit0 =
        Cask2dLayout::FromUri("example://Rube/Adder8Example/Bit0", &opts).expect("layout bit0");
    jeeves_assert_eq!(ctx, bit0.Nodes()[0].Name(), "Bit0");
    jeeves_assert_eq!(ctx, bit0.NodeCount(), 4, "Bit0 must contain HA1, HA2, Or");

    // Karst Submodule Navigation: Host 0 directly under System
    let host0 =
        Cask2dLayout::FromUri("example://Karst/System/Host 0", &opts).expect("layout host0");
    jeeves_assert_eq!(ctx, host0.Nodes()[0].Name(), "Host 0");
    jeeves_assert_eq!(ctx, host0.NodeCount(), 2);

    // Karst Submodule Navigation: MemChan 0 under System with VPU 0
    let mc0 =
        Cask2dLayout::FromUri("example://Karst/System/MemChan 0", &opts).expect("layout memchan0");
    jeeves_assert_eq!(ctx, mc0.Nodes()[0].Name(), "MemChan 0");
    jeeves_assert_eq!(ctx, mc0.NodeCount(), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, CircuitSchematicPinAndWireHitTesting, |ctx| {
    use crate::fenst::cask::Point;
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};
    use crate::silo::USeg;

    let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::CircuitSchematic);
    let full_adder = Cask2dLayout::FromUri("example://Rube/FullAdderExample", &opts)
        .expect("layout FullAdderExample schematic");

    // Must have interactive pins and wire nets
    jeeves_assert!(ctx,
                   full_adder.PinCount() >= 6,
                   "FullAdder schematic must have interactive pins");
    jeeves_assert!(ctx,
                   full_adder.WireCount() >= 2,
                   "FullAdder schematic must have routed wire nets");

    let pins = full_adder.Pins();
    USeg::FromLen(pins.len() as u32).Traverse(|i| {
                                        let pin = &pins[i as usize];
                                        let hit = full_adder.FindPinAt(pin.Point());
                                        jeeves_assert!(ctx,
                                                       hit.is_some(),
                                                       "Exact pin contact point must hit-test");
                                        let hit_pin = hit.unwrap();
                                        jeeves_assert_eq!(ctx,
                                                          hit_pin.ModuleName(),
                                                          pin.ModuleName());
                                        jeeves_assert_eq!(ctx, hit_pin.PortName(), pin.PortName());
                                    });

    // Point outside any pin must return None
    let miss_pin = full_adder.FindPinAt(Point::New(9999.0, 9999.0));
    jeeves_assert!(ctx,
                   miss_pin.is_none(),
                   "Point outside schematic bounds must not hit pin");

    // Wire segment midpoint hit testing
    let wires = full_adder.Wires();
    USeg::FromLen(wires.len() as u32).Traverse(|i| {
                                         let wire = &wires[i as usize];
                                         jeeves_assert!(ctx,
                                                        !wire.Segments().is_empty(),
                                                        "Wire must contain line segments");
                                         let (p1, p2) = wire.Segments()[0];
                                         let mid =
                                             Point::New((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
                                         let hit = full_adder.FindWireAt(mid, 3.0);
                                         jeeves_assert!(ctx,
                                                        hit.is_some(),
                                                        "Wire segment midpoint must hit-test");
                                         jeeves_assert!(ctx,
                                                        !hit.unwrap().NetName().is_empty(),
                                                        "Hit wire must have non-empty NetName");
                                     });

    let miss_wire = full_adder.FindWireAt(Point::New(9999.0, 9999.0), 3.0);
    jeeves_assert!(ctx,
                   miss_wire.is_none(),
                   "Point outside schematic bounds must not hit wire");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(
             Fenst,
             CircuitSchematicConnectedPlacementAndPortSideLayout,
             |ctx| {
                 use crate::fenst::cask2d::{
                     Cask2dLayout, Cask2dLayoutMode, Cask2dOptions, PortSide,
                 };
                 use crate::silo::USeg;

                 let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::CircuitSchematic);
                 let karst_system = Cask2dLayout::FromUri("example://Karst/System", &opts)
        .expect("layout Karst System schematic");

                 // 1. Module Placement & Ordering
                 let km = karst_system.FindNodeByName("KarstModule")
                                      .expect("KarstModule must exist");

                 // Hosts 0..7 must be ordered top-to-bottom on the West of KarstModule
                 let mut prev_host_y = 0.0f32;
                 USeg::FromLen(8).Traverse(|i| {
                     let name = format!("Host {}", i);
                     let host = karst_system.FindNodeByName(&name).expect("Host must exist");
                     jeeves_assert!(ctx,
                                    host.Bounds().x < km.Bounds().x,
                                    "Host must be to the West of KarstModule");
                     jeeves_assert!(ctx,
                                    host.Bounds().y > prev_host_y,
                                    "Hosts must be ordered monotonically top-to-bottom");
                     prev_host_y = host.Bounds().y;
                 });

                 // MemChans 0..7 must be ordered top-to-bottom on the East of KarstModule
                 let mut prev_mc_y = 0.0f32;
                 USeg::FromLen(8).Traverse(|i| {
                     let name = format!("MemChan {}", i);
                     let mc = karst_system.FindNodeByName(&name)
                                          .expect("MemChan must exist");
                     jeeves_assert!(ctx,
                                    mc.Bounds().x > km.Bounds().x,
                                    "MemChan must be to the East of KarstModule");
                     jeeves_assert!(ctx,
                                    mc.Bounds().y > prev_mc_y,
                                    "MemChans must be ordered monotonically top-to-bottom");
                     prev_mc_y = mc.Bounds().y;
                 });

                 // 2. Port Face (West vs East) Placement
                 let pins = karst_system.Pins();

                 // Hosts: both Req and Resp must face East towards KarstModule
                 USeg::FromLen(8).Traverse(|i| {
                     let host_name = format!("Host {}", i);
                     let req_pin =
                         pins.iter()
                             .find(|p| p.ModuleName() == host_name && p.PortName() == "Req");
                     let resp_pin =
                         pins.iter()
                             .find(|p| p.ModuleName() == host_name && p.PortName() == "Resp");
                     jeeves_assert!(ctx, req_pin.is_some(), "Host Req pin must exist");
                     jeeves_assert!(ctx, resp_pin.is_some(), "Host Resp pin must exist");
                     jeeves_assert_eq!(ctx,
                                       req_pin.unwrap().Side(),
                                       PortSide::East,
                                       "Host Req must face East towards KarstModule");
                     jeeves_assert_eq!(ctx,
                                       resp_pin.unwrap().Side(),
                                       PortSide::East,
                                       "Host Resp must face East towards KarstModule");
                 });

                 // MemChans: both Req and Resp must face West towards KarstModule
                 USeg::FromLen(8).Traverse(|i| {
                     let mc_name = format!("MemChan {}", i);
                     let req_pin =
                         pins.iter()
                             .find(|p| p.ModuleName() == mc_name && p.PortName() == "Req");
                     let resp_pin =
                         pins.iter()
                             .find(|p| p.ModuleName() == mc_name && p.PortName() == "Resp");
                     jeeves_assert!(ctx, req_pin.is_some(), "MemChan Req pin must exist");
                     jeeves_assert!(ctx, resp_pin.is_some(), "MemChan Resp pin must exist");
                     jeeves_assert_eq!(ctx,
                                       req_pin.unwrap().Side(),
                                       PortSide::West,
                                       "MemChan Req must face West towards KarstModule");
                     jeeves_assert_eq!(ctx,
                                       resp_pin.unwrap().Side(),
                                       PortSide::West,
                                       "MemChan Resp must face West towards KarstModule");
                 });

                 // KarstModule: Host ports and boundary inputs face West; MC ports and boundary outputs face East
                 USeg::FromLen(8).Traverse(|i| {
                                     let in_name = format!("Host_In_{}", i);
                                     let out_name = format!("Host_Out_{}", i);
                                     let in_pin = pins.iter().find(|p| {
                                                                 p.ModuleName() == "KarstModule"
                                                                 && p.PortName() == in_name
                                                             });
                                     let out_pin = pins.iter().find(|p| {
                                                                  p.ModuleName() == "KarstModule"
                                                                  && p.PortName() == out_name
                                                              });
                                     jeeves_assert!(ctx,
                                                    in_pin.is_some(),
                                                    "KarstModule Host_In must exist");
                                     jeeves_assert!(ctx,
                                                    out_pin.is_some(),
                                                    "KarstModule Host_Out must exist");
                                     jeeves_assert_eq!(ctx,
                                                       in_pin.unwrap().Side(),
                                                       PortSide::West,
                                                       "Host_In must face West towards Hosts");
                                     jeeves_assert_eq!(ctx,
                                                       out_pin.unwrap().Side(),
                                                       PortSide::West,
                                                       "Host_Out must face West towards Hosts");

                                     let mc_in_name = format!("MC_In_{}", i);
                                     let mc_out_name = format!("MC_Out_{}", i);
                                     let mc_in_pin = pins.iter().find(|p| {
                                                                    p.ModuleName() == "KarstModule"
                                                                    && p.PortName() == mc_in_name
                                                                });
                                     let mc_out_pin = pins.iter().find(|p| {
                                                                     p.ModuleName() == "KarstModule"
                                                                     && p.PortName() == mc_out_name
                                                                 });
                                     jeeves_assert!(ctx,
                                                    mc_in_pin.is_some(),
                                                    "KarstModule MC_In must exist");
                                     jeeves_assert!(ctx,
                                                    mc_out_pin.is_some(),
                                                    "KarstModule MC_Out must exist");
                                     jeeves_assert_eq!(ctx,
                                                       mc_in_pin.unwrap().Side(),
                                                       PortSide::East,
                                                       "MC_In must face East towards MemChans");
                                     jeeves_assert_eq!(ctx,
                                                       mc_out_pin.unwrap().Side(),
                                                       PortSide::East,
                                                       "MC_Out must face East towards MemChans");
                                 });

                 // 3. Direct Channel Manhattan Wire Routing (3 segments each)
                 let wires = karst_system.Wires();
                 USeg::FromLen(wires.len() as u32).Traverse(|i| {
                     let w = &wires[i as usize];
                     if (w.DriverModule().starts_with("Host ") && w.SinkModule() == "KarstModule")
                        || (w.DriverModule() == "KarstModule"
                            && w.SinkModule().starts_with("Host "))
                     {
                         jeeves_assert_eq!(ctx,
                                           w.Segments().len(),
                                           3,
                                           "Inter-column wire must route directly with 3 segments");
                     }
                     if (w.DriverModule() == "KarstModule"
                         && w.SinkModule().starts_with("MemChan "))
                        || (w.DriverModule().starts_with("MemChan ")
                            && w.SinkModule() == "KarstModule")
                     {
                         jeeves_assert_eq!(ctx,
                                           w.Segments().len(),
                                           3,
                                           "Inter-column wire must route directly with 3 segments");
                     }
                 });
             }
);

//-------------------------------------------------------------------------------------------------

jeeves_test!(
             Fenst,
             CircuitSchematicSubmoduleExplorationPortPlacementInvariance,
             |ctx| {
                 use crate::fenst::cask2d::{
                     Cask2dLayout, Cask2dLayoutMode, Cask2dOptions, PortSide,
                 };
                 use crate::silo::USeg;

                 let opts = Cask2dOptions::New().WithMode(Cask2dLayoutMode::CircuitSchematic);

                 // 1. Exploring leaf submodule Host 0 directly via URI:
                 // In Karst/System parent, Host 0 was placed on West of KarstModule, with Req and Resp on East face.
                 // When explored directly, both Req and Resp must remain on East face!
                 let host0_layout = Cask2dLayout::FromUri("example://Karst/System/Host 0", &opts)
        .expect("layout Host 0 submodule schematic");
                 let host0_pins = host0_layout.Pins();

                 let req_pin = host0_pins.iter()
                                         .find(|p| p.PortName() == "Req")
                                         .expect("Req pin exists");
                 let resp_pin = host0_pins.iter()
                                          .find(|p| p.PortName() == "Resp")
                                          .expect("Resp pin exists");
                 jeeves_assert_eq!(ctx,
                                   req_pin.Side(),
                                   PortSide::East,
                                   "Explored Host 0: Req must remain on East face as decided in parent");
                 jeeves_assert_eq!(ctx,
                                   resp_pin.Side(),
                                   PortSide::East,
                                   "Explored Host 0: Resp must remain on East face as decided in parent");

                 // 2. Exploring leaf submodule MemChan 0 directly via URI:
                 // In Karst/System parent, MemChan 0 was placed on East of KarstModule, with Req and Resp on West face.
                 // When explored directly, both Req and Resp must remain on West face!
                 let mc0_layout = Cask2dLayout::FromUri("example://Karst/System/MemChan 0", &opts)
        .expect("layout MemChan 0 submodule schematic");
                 let mc0_pins = mc0_layout.Pins();

                 let mc_req_pin = mc0_pins.iter()
                                          .find(|p| p.PortName() == "Req")
                                          .expect("Req pin exists");
                 let mc_resp_pin = mc0_pins.iter()
                                           .find(|p| p.PortName() == "Resp")
                                           .expect("Resp pin exists");
                 jeeves_assert_eq!(ctx,
                                   mc_req_pin.Side(),
                                   PortSide::West,
                                   "Explored MemChan 0: Req must remain on West face as decided in parent");
                 jeeves_assert_eq!(ctx,
                                   mc_resp_pin.Side(),
                                   PortSide::West,
                                   "Explored MemChan 0: Resp must remain on West face as decided in parent");

                 // 3. Exploring container submodule KarstModule directly via URI:
                 // In Karst/System parent, KarstModule had Host ports on West and MC ports on East.
                 // When explored directly as a container, its boundary ports on the outer canvas frame must retain:
                 // - Host_In_0..7 and Host_Out_0..7 on West boundary
                 // - MC_In_0..7 and MC_Out_0..7 on East boundary
                 let km_layout = Cask2dLayout::FromUri("example://Karst/System/KarstModule", &opts)
        .expect("layout KarstModule submodule schematic");
                 let km_pins = km_layout.Pins();

                 USeg::FromLen(8).Traverse(|i| {
        let in_name = format!("Host_In_{}", i);
        let out_name = format!("Host_Out_{}", i);
        let in_pin = km_pins.iter().find(|p| p.PortName() == in_name).expect("Host_In exists");
        let out_pin = km_pins.iter().find(|p| p.PortName() == out_name).expect("Host_Out exists");
        jeeves_assert_eq!(ctx, in_pin.Side(), PortSide::West, "Explored KarstModule: Host_In must remain on West boundary");
        jeeves_assert_eq!(ctx, out_pin.Side(), PortSide::West, "Explored KarstModule: Host_Out must remain on West boundary");

        let mc_in_name = format!("MC_In_{}", i);
        let mc_out_name = format!("MC_Out_{}", i);
        let mc_in_pin = km_pins.iter().find(|p| p.PortName() == mc_in_name).expect("MC_In exists");
        let mc_out_pin = km_pins.iter().find(|p| p.PortName() == mc_out_name).expect("MC_Out exists");
        jeeves_assert_eq!(ctx, mc_in_pin.Side(), PortSide::East, "Explored KarstModule: MC_In must remain on East boundary");
        jeeves_assert_eq!(ctx, mc_out_pin.Side(), PortSide::East, "Explored KarstModule: MC_Out must remain on East boundary");
    });

                 let clk_pin = km_pins.iter()
                                      .find(|p| p.PortName() == "SysClk")
                                      .expect("SysClk exists");
                 let rst_pin = km_pins.iter()
                                      .find(|p| p.PortName() == "Reset_n")
                                      .expect("Reset_n exists");
                 let status_pin = km_pins.iter()
                                         .find(|p| p.PortName() == "Status")
                                         .expect("Status exists");
                 let int_pin = km_pins.iter()
                                      .find(|p| p.PortName() == "Interrupt")
                                      .expect("Interrupt exists");
                 jeeves_assert_eq!(ctx,
                                   clk_pin.Side(),
                                   PortSide::West,
                                   "Explored KarstModule: SysClk on West boundary");
                 jeeves_assert_eq!(ctx,
                                   rst_pin.Side(),
                                   PortSide::West,
                                   "Explored KarstModule: Reset_n on West boundary");
                 jeeves_assert_eq!(ctx,
                                   status_pin.Side(),
                                   PortSide::East,
                                   "Explored KarstModule: Status on East boundary");
                 jeeves_assert_eq!(ctx,
                                   int_pin.Side(),
                                   PortSide::East,
                                   "Explored KarstModule: Interrupt on East boundary");
             }
);

//-------------------------------------------------------------------------------------------------
// Planar Sketch Document, Entities, and Bounds Tests
jeeves_test!(Fenst, SketchDocumentPrimitiveCreationAndBounds, |ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    jeeves_assert_eq!(ctx, doc.EntityCount(), 0);
    jeeves_assert_eq!(ctx, doc.Revision(), 0);

    // 1. Line addition
    let l_id = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 50.0))
                  .expect("valid line");
    jeeves_assert_eq!(ctx, l_id, 1);
    jeeves_assert_eq!(ctx, doc.EntityCount(), 1);

    // 2. Circle addition
    let c_id = doc.AddCircle(SketchPoint::New(50.0, 50.0), 25.0)
                  .expect("valid circle");
    jeeves_assert_eq!(ctx, c_id, 2);

    // 3. Arc addition
    let a_id = doc.AddArc(SketchPoint::New(0.0, 0.0), 10.0, 0.0, std::f64::consts::PI)
                  .expect("valid arc");
    jeeves_assert_eq!(ctx, a_id, 3);

    // 4. Ellipse addition
    let e_id = doc.AddEllipse(SketchPoint::New(200.0, 100.0), 40.0, 20.0, 0.0)
                  .expect("valid ellipse");
    jeeves_assert_eq!(ctx, e_id, 4);

    // 5. Polyline addition
    let pts = [SketchPoint::New(0.0, 0.0),
               SketchPoint::New(20.0, 0.0),
               SketchPoint::New(10.0, 15.0)];
    let p_id = doc.AddPolyline((&pts).into(), true)
                  .expect("valid polyline");
    jeeves_assert_eq!(ctx, p_id, 5);
    jeeves_assert_eq!(ctx, doc.EntityCount(), 5);

    // Verify overall bounds
    let bounds = doc.ComputeBounds();
    jeeves_assert!(ctx, bounds.IsValid());
    jeeves_assert!(ctx, bounds._Min._X <= -10.0);
    jeeves_assert!(ctx, bounds._Max._X >= 240.0);

    // Verify parameter rejection
    jeeves_assert!(ctx, doc.AddCircle(SketchPoint::New(0.0, 0.0), 0.0).is_err());
    jeeves_assert!(ctx,
                   doc.AddCircle(SketchPoint::New(0.0, 0.0), -5.0).is_err());
    jeeves_assert!(ctx,
                   doc.AddLine(SketchPoint::New(10.0, 10.0), SketchPoint::New(10.0, 10.0))
                      .is_err());
    jeeves_assert!(ctx,
                   doc.AddArc(SketchPoint::New(0.0, 0.0), 10.0, 0.0, 0.0)
                      .is_err());
    jeeves_assert!(ctx,
                   doc.AddArc(SketchPoint::New(0.0, 0.0),
                              10.0,
                              0.0,
                              3.0 * std::f64::consts::PI)
                      .is_err());
});

//-------------------------------------------------------------------------------------------------
// Entity Removal and Slot Reindexing
jeeves_test!(Fenst, SketchDocumentEntityRemovalAndSlotReindexing, |ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    let id1 = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(10.0, 0.0))
                 .unwrap();
    let id2 = doc.AddLine(SketchPoint::New(10.0, 0.0), SketchPoint::New(20.0, 0.0))
                 .unwrap();
    let id3 = doc.AddLine(SketchPoint::New(20.0, 0.0), SketchPoint::New(30.0, 0.0))
                 .unwrap();

    jeeves_assert_eq!(ctx, doc.EntityCount(), 3);
    jeeves_assert!(ctx, doc.FindEntity(id2).is_some());

    // Remove middle entity
    let removed = doc.RemoveEntity(id2).expect("remove id2");
    jeeves_assert_eq!(ctx, removed.Id(), id2);
    jeeves_assert_eq!(ctx, doc.EntityCount(), 2);

    // Binary search lookup should succeed for id1 and id3, and return None for id2
    jeeves_assert!(ctx, doc.FindEntity(id1).is_some());
    jeeves_assert!(ctx, doc.FindEntity(id2).is_none());
    jeeves_assert!(ctx, doc.FindEntity(id3).is_some());
});

//-------------------------------------------------------------------------------------------------
// Sketch History Transactional Undo/Redo & Dirty Tracking
jeeves_test!(
             Fenst,
             SketchHistoryTransactionalUndoRedoAndDirtyTracking,
             |ctx| {
                 use crate::fenst::sketch::{
                     LineSegment, SketchDocument, SketchEntity, SketchPoint, SketchPrimitive,
                 };
                 use crate::fenst::sketch_edit::{SketchHistory, SketchPatchBuilder};

                 let mut doc = SketchDocument::New();
                 let mut history = SketchHistory::New(32);
                 jeeves_assert!(ctx, !history.CanUndo());
                 jeeves_assert!(ctx, !history.CanRedo());
                 jeeves_assert!(ctx, !history.IsDirty(doc.Revision()));

                 // Construct a patch with an added line
                 let line_ent = SketchEntity::New(
        100,
        SketchPrimitive::Line(LineSegment::New(SketchPoint::New(0.0, 0.0), SketchPoint::New(50.0, 50.0))),
    );
                 let mut builder = SketchPatchBuilder::New();
                 builder.AddEntity(line_ent);
                 let patch = builder.Build();

                 // Commit patch
                 let _change_set = history.CommitPatch(&mut doc, patch).expect("commit patch");
                 jeeves_assert_eq!(ctx, doc.EntityCount(), 1);
                 jeeves_assert!(ctx, history.CanUndo());
                 jeeves_assert!(ctx, !history.CanRedo());
                 jeeves_assert!(ctx, history.IsDirty(doc.Revision()));

                 // Mark as saved
                 history.MarkSaved(doc.Revision());
                 jeeves_assert!(ctx, !history.IsDirty(doc.Revision()));

                 // Undo
                 let _undo_cs = history.Undo(&mut doc)
                                       .expect("undo")
                                       .expect("some change set");
                 jeeves_assert_eq!(ctx, doc.EntityCount(), 0);
                 jeeves_assert!(ctx, !history.CanUndo());
                 jeeves_assert!(ctx, history.CanRedo());
                 jeeves_assert!(ctx, history.IsDirty(doc.Revision()));

                 // Redo
                 let _redo_cs = history.Redo(&mut doc)
                                       .expect("redo")
                                       .expect("some change set");
                 jeeves_assert_eq!(ctx, doc.EntityCount(), 1);
                 jeeves_assert!(ctx, history.CanUndo());
                 jeeves_assert!(ctx, !history.CanRedo());
             }
);

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fenst, SketchXplrHierarchyAndLeafInspection, |ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::fenst::sketch_xplr::SketchXplr;
    use crate::fenst::xplr::Xplr;

    let mut doc = SketchDocument::New();
    let p1 = SketchPoint::New(0.0, 0.0);
    let p2 = SketchPoint::New(100.0, 0.0);
    let id1 = doc.AddLine(p1, p2).expect("add line");
    let id2 = doc.AddCircle(SketchPoint::New(50.0, 50.0), 25.0)
                 .expect("add circle");

    let xplr = SketchXplr::FromDocument(&doc, "TestSketch", "example://sketch/test");
    jeeves_assert_eq!(ctx, xplr.Name(), "TestSketch");
    jeeves_assert_eq!(ctx, xplr.Path(), "example://sketch/test");
    jeeves_assert!(ctx, !xplr.IsLeaf());

    let branch = xplr.Branch().expect("branch");
    let children = branch.Children().expect("children");
    jeeves_assert_eq!(ctx, children.Len(), 3);

    // Verify 3 top-level branches: Layers, Groups, Entities
    let layers_child = children.Arr().Get(0).unwrap();
    let groups_child = children.Arr().Get(1).unwrap();
    let entities_child = children.Arr().Get(2).unwrap();

    jeeves_assert_eq!(ctx, layers_child.Name(), "Layers");
    jeeves_assert_eq!(ctx, groups_child.Name(), "Groups");
    jeeves_assert_eq!(ctx, entities_child.Name(), "Entities");

    // Inspect entities branch
    let ent_branch = entities_child.Branch().expect("entities branch");
    let ent_children = ent_branch.Children().expect("entity children");
    jeeves_assert_eq!(ctx, ent_children.Len(), 2);

    let ent1 = ent_children.Arr().Get(0).unwrap();
    let ent2 = ent_children.Arr().Get(1).unwrap();
    jeeves_assert!(ctx, ent1.IsLeaf());
    jeeves_assert!(ctx, ent2.IsLeaf());
    jeeves_assert!(ctx, ent1.Name().contains(&format!("Line {}", id1)));
    jeeves_assert!(ctx, ent2.Name().contains(&format!("Circle {}", id2)));
});

//-------------------------------------------------------------------------------------------------
