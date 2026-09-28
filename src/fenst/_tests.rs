//-- _tests.rs ------------------------------------------------------------------------------------------------------
use	crate::fenst::{ BranchXplr, FsBranch, FsLeaf, LeafXplr, Xplr, XplrRegistry };
use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, FilesystemExplorer, |ctx| {
    let  	branch = FsBranch::New( "src".to_string());
    let  	children = branch.Children().expect( "src should be readable");
    jeeves_assert!( ctx, !children.IsEmpty());
    let  	leaf = FsLeaf::New( "Cargo.toml".to_string());
    jeeves_assert!( ctx, leaf.IsLeaf());
    jeeves_assert_eq!( ctx, leaf.Extension(), "toml");
    let  	chunk = leaf
        .ReadChunk( 0, 64)
        .expect( "Cargo.toml should be readable");
    jeeves_assert!( ctx, chunk.Content().contains( "[package]"));
    jeeves_assert!( ctx, !chunk.IsEof());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, ProviderRegistry, |ctx| {
    let  	registry = XplrRegistry::New();
    let  	( scheme, root) = registry
        .OpenRoot( "file://src")
        .expect( "file provider should open src");
    jeeves_assert_eq!( ctx, scheme, "file");
    jeeves_assert_eq!( ctx, root.Name(), "src");
    jeeves_assert!( ctx, root.ChildCount().expect( "src count") > 0);
    jeeves_assert!( ctx, registry.OpenRoot( "missing://root").is_err());
});

//---------------------------------------------------------------------------------------------------------------------------------

struct MockLeaf( &'static str);
impl Xplr for MockLeaf {
    fn	Name( &self) -> &str { self.0 }
    fn	Path( &self) -> &str { self.0 }
    fn	IsLeaf( &self) -> bool { true }
}

struct MockBranch {
    name:        &'static str,
    children_fn: fn() -> crate::silo::Buff< Box< dyn Xplr>>,
}
impl Xplr for MockBranch {
    fn	Name( &self) -> &str { self.name }
    fn	Path( &self) -> &str { self.name }
    fn	IsLeaf( &self) -> bool { false }
    fn	Branch( &self) -> Option< &dyn BranchXplr> { Some( self) }
}
impl BranchXplr for MockBranch {
    fn	Children( &self) -> Result< crate::silo::Buff< Box< dyn Xplr>>, String>
    {
        Ok( ( self.children_fn)())
    }
    fn	ChildCount( &self) -> Result< u32, String>
    {
        Ok( ( self.children_fn)().Len())
    }
}

fn	BuildMockTree() -> MockBranch
{
    fn	make_dir_a() -> crate::silo::Buff< Box< dyn Xplr>>
    {
        let  	mut s = crate::silo::Stash::New();
        s.Push( Box::new( MockLeaf( "FileA1")) as Box< dyn Xplr>);
        s.Push( Box::new( MockLeaf( "FileA2")) as Box< dyn Xplr>);
        s.IntoBuff()
    }
    fn	make_dir_b() -> crate::silo::Buff< Box< dyn Xplr>>
    {
        let  	mut s = crate::silo::Stash::New();
        s.Push( Box::new( MockLeaf( "FileB1")) as Box< dyn Xplr>);
        s.IntoBuff()
    }
    fn	make_root() -> crate::silo::Buff< Box< dyn Xplr>>
    {
        let  	mut s = crate::silo::Stash::New();
        s.Push( Box::new( MockBranch { name: "DirA", children_fn: make_dir_a }) as Box< dyn Xplr>);
        s.Push( Box::new( MockBranch { name: "DirB", children_fn: make_dir_b }) as Box< dyn Xplr>);
        s.Push( Box::new( MockLeaf( "FileRoot")) as Box< dyn Xplr>);
        s.IntoBuff()
    }
    MockBranch { name: "Root", children_fn: make_root }
}

jeeves_test!( Fenst, TraverseDepthFullOrder, |ctx| {
    let  	root = BuildMockTree();
    let  	mut events: Vec< (String, bool)> = Vec::new();
    let  	mut ancestor_paths: Vec< String> = Vec::new();

    root.TraverseDepth( |ancestors, enter| {
        let  	curr = ancestors.last().expect( "must have current node");
        events.push( ( curr.Name().to_string(), enter));

        let  	path = ancestors.iter().map( |a| a.Name()).collect::< Vec< _>>().join( "/");
        if enter {
            ancestor_paths.push( path);
        }
        true
    });

    // Check complete preorder enter / postorder exit event sequence
    let  	expected_events = [
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
    jeeves_assert_eq!( ctx, events.len(), expected_events.len());
    for (i, expected) in expected_events.iter().enumerate() {
        jeeves_assert_eq!( ctx, &events[i], expected);
    }

    // Check ancestor paths
    let  	expected_paths = [
        "Root",
        "Root/DirA",
        "Root/DirA/FileA1",
        "Root/DirA/FileA2",
        "Root/DirB",
        "Root/DirB/FileB1",
        "Root/FileRoot",
    ];
    jeeves_assert_eq!( ctx, ancestor_paths.len(), expected_paths.len());
    for (i, expected) in expected_paths.iter().enumerate() {
        jeeves_assert_eq!( ctx, &ancestor_paths[i], expected);
    }
});

jeeves_test!( Fenst, TraverseDepthPruneOnEntry, |ctx| {
    let  	root = BuildMockTree();
    let  	mut visited = Vec::new();

    root.TraverseDepth( |ancestors, enter| {
        let  	curr = ancestors.last().unwrap();
        visited.push( ( curr.Name().to_string(), enter));
        // Prune DirA on entry: should treat DirA as a leaf-node (no children, no exit call)
        if curr.Name() == "DirA" && enter {
            return false;
        }
        true
    });

    let  	expected = [
        ( "Root".to_string(), true),
        ( "DirA".to_string(), true),   // Pruned on entry: treated as leaf, no exit call!
        ( "DirB".to_string(), true),
        ( "FileB1".to_string(), true),
        ( "DirB".to_string(), false),
        ( "FileRoot".to_string(), true),
        ( "Root".to_string(), false),
    ];
    jeeves_assert_eq!( ctx, visited.len(), expected.len());
    for (i, exp) in expected.iter().enumerate() {
        jeeves_assert_eq!( ctx, &visited[i], exp);
    }
});

jeeves_test!( Fenst, TraverseDepthAbortOnExit, |ctx| {
    let  	root = BuildMockTree();
    let  	mut visited = Vec::new();

    root.TraverseDepth( |ancestors, enter| {
        let  	curr = ancestors.last().unwrap();
        visited.push( ( curr.Name().to_string(), enter));
        // Abort traversal on exit of DirA
        if curr.Name() == "DirA" && !enter {
            return false;
        }
        true
    });

    // Traversal should abort immediately upon DirA exit returning false.
    // DirB, FileRoot, and Root exit must never be visited.
    let  	expected = [
        ( "Root".to_string(), true),
        ( "DirA".to_string(), true),
        ( "FileA1".to_string(), true),
        ( "FileA2".to_string(), true),
        ( "DirA".to_string(), false),  // Aborted here
    ];
    jeeves_assert_eq!( ctx, visited.len(), expected.len());
    for (i, exp) in expected.iter().enumerate() {
        jeeves_assert_eq!( ctx, &visited[i], exp);
    }
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskLabelIntrinsicMeasurement, |ctx| {
    use crate::fenst::cask::{ Cask, LayoutAndRenderCask, Rect };

    let  	label = Cask::NewLabel( "msg", "Hello")
        .WithFontSize( 10.0);
    let  	(w, h) = label.MeasureIntrinsic();
    // 5 chars * 10.0 * 0.55 = 27.5, line height = 10.0 * 1.25 = 12.5
    jeeves_assert_eq!( ctx, w, 27.5);
    jeeves_assert_eq!( ctx, h, 12.5);

    let  	commands = LayoutAndRenderCask( &label, 50.0, 100.0);
    jeeves_assert_eq!( ctx, commands.len(), 1);
    jeeves_assert_eq!( ctx, label.Bounds(), Rect::New( 50.0, 100.0, 27.5, 12.5));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskHorizontalLayoutWithPaddingAndGap, |ctx| {
    use crate::fenst::cask::{ Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing };

    let  	root = Cask::NewWindow( "row")
        .WithDirection( LayoutDirection::LeftToRight)
        .WithPadding( Padding::Axes( 10.0, 5.0))
        .WithChildGap( 8.0)
        .WithChild(
            Cask::NewWindow( "c1")
                .WithSize( Sizing::Fixed( 100.0), Sizing::Fixed( 50.0))
        )
        .WithChild(
            Cask::NewWindow( "c2")
                .WithSize( Sizing::Fixed( 200.0), Sizing::Fixed( 60.0))
        );

    let  	_ = LayoutAndRenderCask( &root, 0.0, 0.0);

    // Parent fit: width = 100 + 200 + 8 (gap) + 20 (padding) = 328.0
    // Parent fit: height = max(50, 60) + 10 (padding) = 70.0
    jeeves_assert_eq!( ctx, root.Bounds(), Rect::New( 0.0, 0.0, 328.0, 70.0));

    let  	children = root.Children();
    // c1 bounds: x = 10.0, y = 5.0, w = 100.0, h = 50.0
    jeeves_assert_eq!( ctx, children[0].Bounds(), Rect::New( 10.0, 5.0, 100.0, 50.0));
    // c2 bounds: x = 10.0 + 100.0 + 8.0 = 118.0, y = 5.0, w = 200.0, h = 60.0
    jeeves_assert_eq!( ctx, children[1].Bounds(), Rect::New( 118.0, 5.0, 200.0, 60.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskVerticalLayoutWithPaddingAndGap, |ctx| {
    use crate::fenst::cask::{ Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing };

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

    let  	_ = LayoutAndRenderCask( &root, 100.0, 50.0);

    // Parent fit: width = max(150, 180) + 24 (padding) = 204.0
    // Parent fit: height = 30 + 40 + 4 (gap) + 24 (padding) = 98.0
    jeeves_assert_eq!( ctx, root.Bounds(), Rect::New( 100.0, 50.0, 204.0, 98.0));

    let  	children = root.Children();
    // item1: x = 112.0, y = 62.0, w = 150.0, h = 30.0
    jeeves_assert_eq!( ctx, children[0].Bounds(), Rect::New( 112.0, 62.0, 150.0, 30.0));
    // item2: x = 112.0, y = 62.0 + 30.0 + 4.0 = 96.0, w = 180.0, h = 40.0
    jeeves_assert_eq!( ctx, children[1].Bounds(), Rect::New( 112.0, 96.0, 180.0, 40.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskGrowDistribution, |ctx| {
    use crate::fenst::cask::{ Cask, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing };

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

    let  	_ = LayoutAndRenderCask( &root, 0.0, 0.0);

    // Inner width = 500 - 20 = 480.
    // Non-grow width = 100. Gaps = 2 * 10 = 20.
    // Remaining = 480 - 100 - 20 = 360.
    // Each of the 2 grow children gets 360 / 2 = 180.0 width!
    // Inner height = 300 - 20 = 280.
    // grow_child_1 cross grow height = 280.0.

    let  	children = root.Children();
    // fixed_child: x = 10.0, y = 10.0, w = 100.0, h = 50.0
    jeeves_assert_eq!( ctx, children[0].Bounds(), Rect::New( 10.0, 10.0, 100.0, 50.0));
    // grow_child_1: x = 10 + 100 + 10 = 120.0, y = 10.0, w = 180.0, h = 280.0
    jeeves_assert_eq!( ctx, children[1].Bounds(), Rect::New( 120.0, 10.0, 180.0, 280.0));
    // grow_child_2: x = 120 + 180 + 10 = 310.0, y = 10.0, w = 180.0, h = 60.0
    jeeves_assert_eq!( ctx, children[2].Bounds(), Rect::New( 310.0, 10.0, 180.0, 60.0));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskNestedClayWindowRenderCommands, |ctx| {
    use crate::fenst::cask::{ Cask, CaskRenderCommand, Color, LayoutAndRenderCask, LayoutDirection, Padding, Rect, Sizing };

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

    let  	commands = LayoutAndRenderCask( &root, 10.0, 20.0);

    // Verify root window bounds
    jeeves_assert_eq!( ctx, root.Bounds(), Rect::New( 10.0, 20.0, 400.0, 300.0));

    // Verify TitleBar: x = 14.0, y = 24.0, w = 400 - 8 = 392.0, h = 32.0
    let  	title_bar = &root.Children()[0];
    jeeves_assert_eq!( ctx, title_bar.Bounds(), Rect::New( 14.0, 24.0, 392.0, 32.0));

    // Verify Body: x = 14.0, y = 24.0 + 32.0 + 4.0 = 60.0, w = 392.0, h = 300 - 8 - 32 - 4 = 256.0
    let  	body = &root.Children()[1];
    jeeves_assert_eq!( ctx, body.Bounds(), Rect::New( 14.0, 60.0, 392.0, 256.0));

    // Verify Sidebar: x = 14.0, y = 60.0, w = 100.0, h = 256.0
    let  	sidebar = &body.Children()[0];
    jeeves_assert_eq!( ctx, sidebar.Bounds(), Rect::New( 14.0, 60.0, 100.0, 256.0));

    // Verify Content: x = 14.0 + 100.0 + 4.0 = 118.0, y = 60.0, w = 392 - 100 - 4 = 288.0, h = 256.0
    let  	content = &body.Children()[1];
    jeeves_assert_eq!( ctx, content.Bounds(), Rect::New( 118.0, 60.0, 288.0, 256.0));

    // Check emitted commands count and types
    jeeves_assert!( ctx, !commands.is_empty());

    // Verify first command is root Window background
    match &commands[0] {
        CaskRenderCommand::Rectangle { bounds, color, .. } => {
            jeeves_assert_eq!( ctx, *bounds, Rect::New( 10.0, 20.0, 400.0, 300.0));
            jeeves_assert_eq!( ctx, *color, Color::Hex( 0x1E1E2E));
        }
        _ => panic!( "Expected Rectangle command for root background"),
    }

    // Verify second command is root Window border
    match &commands[1] {
        CaskRenderCommand::Border { bounds, color, width, .. } => {
            jeeves_assert_eq!( ctx, *bounds, Rect::New( 10.0, 20.0, 400.0, 300.0));
            jeeves_assert_eq!( ctx, *color, Color::Hex( 0x45475A));
            jeeves_assert_eq!( ctx, *width, 1.0);
        }
        _ => panic!( "Expected Border command for root border"),
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fenst, CaskSceneLayoutInvariants, |ctx| {
    use crate::fenst::cask::Cask;
    use crate::fenst::cask_scene::CaskScene;
    use crate::silo::USeg;
    let mut branch = Cask::NewWindow( "deep leaf");
    USeg::FromLen( 6).Traverse( |i| branch = Cask::NewWindow( format!( "level {i}")).WithChild( std::mem::replace( &mut branch, Cask::NewWindow( "unused"))));
    let mut root = Cask::NewWindow( "root").WithChild( branch);
    USeg::FromLen( 20).Traverse( |i| root = std::mem::replace( &mut root, Cask::NewWindow( "unused")).WithChild( Cask::NewWindow( format!( "sibling {i}"))));
    let mut scene = CaskScene::FromRoot( &root);
    scene.Layout( |name| [name.len() as f32 * 7.0, 20.0]);
    let nodes = scene.Nodes();
    jeeves_assert_eq!( ctx, scene.MaxDepth(), 8);
    jeeves_assert_eq!( ctx, nodes.Size(), 28);
    let mut zVariation = false;
    nodes.USeg().Traverse( |i| {
                    let node = &nodes[i];
                    let p = node.Origin();
                    let size = node.Size();
                    if node.IsLeaf() {
                        jeeves_assert_eq!( ctx, size[1], size[2]);
                    }
                    if node.Parent() != u32::MAX {
                        let parent = &nodes[node.Parent()];
                        USeg::FromLen( 3).Traverse( |axis| {
                                            let a = axis as usize;
                                            jeeves_assert!( ctx, p[a] >= parent.Origin()[a]);
                                            jeeves_assert!( ctx,
                                                           p[a] + size[a]
                                                           <= parent.Origin()[a]
                                                              + parent.Size()[a]
                                                              + 0.001);
                                        });
                    }
                    USeg::WithLen( i + 1, nodes.Size() - i - 1).Traverse( |j| {
                        let other = &nodes[j];
                        if node.Depth() != other.Depth() {
                            return;
                        }
                        let mut disjoint = false;
                        USeg::FromLen( 3).Traverse( |axis| {
                                            let a = axis as usize;
                                            disjoint |= p[a] + size[a] <= other.Origin()[a]
                                                        || other.Origin()[a] + other.Size()[a]
                                                           <= p[a];
                                        });
                        jeeves_assert!( ctx, disjoint, "Same-level casks must be disjoint");
                        if node.Parent() == other.Parent() {
                            zVariation |= p[2] != other.Origin()[2];
                        }
                    });
                });
    jeeves_assert!( ctx, zVariation, "Packing must use Z as well as Y");
    let bounds = scene.Bounds();
    scene.Layout( |name| [name.len() as f32 * 7.0, 20.0]);
    jeeves_assert_eq!( ctx, bounds, scene.Bounds());
    let labels = Cask::NewWindow( "labels")
        .WithChild( Cask::NewLabel( "first", "First"))
        .WithChild( Cask::NewLabel( "second", "Second"));
    let labelled = CaskScene::FromRoot( &labels);
    let nodes = labelled.Nodes();
    jeeves_assert_eq!( ctx, nodes[0].Name(), "First\nSecond");
});

jeeves_test!( Fenst, CaskSceneReadsAllDepths, |ctx| {
    use crate::fenst::cask_scene::CaskScene;
    use crate::silo::{IArr, USeg};
    use std::sync::atomic::AtomicBool;
    // Unique, test-owned directory. Drop removes only this fixture, including on panic.
    struct Fixture( std::path::PathBuf);
    impl Drop for Fixture
    {
        fn drop( &mut self) { let _ = std::fs::remove_dir_all( &self.0); }
    }
    let path = std::env::temp_dir().join( format!(
        "trellis-cask-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since( std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos()
    ));
    std::fs::create_dir( &path).unwrap();
    let fixture = Fixture( path);
    let mut leaf = fixture.0.clone();
    USeg::FromLen( 7).Traverse( |i| {
                        leaf.push( format!( "d{i}"));
                        std::fs::create_dir( &leaf).unwrap();
                    });
    std::fs::write( leaf.join( "leaf.txt"), "leaf").unwrap();
    USeg::FromLen( 9).Traverse( |i| {
                        std::fs::write( fixture.0.join( format!( "sibling-{i}.txt")), "").unwrap()
                    });
    let scene = CaskScene::Read( &fixture.0, &AtomicBool::new( false)).unwrap();
    jeeves_assert_eq!( ctx, scene.MaxDepth(), 9);
    jeeves_assert_eq!( ctx, scene.Nodes().Size(), 18);
    let mut deepest = 0;
    scene.Nodes()
         .Traverse( |node| deepest = deepest.max( node.Depth()));
    jeeves_assert_eq!( ctx, deepest, scene.MaxDepth());
    jeeves_assert_eq!( ctx, scene.Nodes()[0].Height(), 8);
    scene.Nodes().Traverse( |node| {
        if node.IsLeaf() {
            jeeves_assert_eq!( ctx, node.Height(), 0);
        }
    });
    jeeves_assert!( ctx,
                   CaskScene::Read( &fixture.0, &AtomicBool::new( true)).is_err());
    jeeves_assert!( ctx,
                   CaskScene::Read( &fixture.0.join( "missing"), &AtomicBool::new( false)).is_err());
});

//-------------------------------------------------------------------------------------------------
