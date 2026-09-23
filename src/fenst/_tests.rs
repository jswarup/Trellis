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
