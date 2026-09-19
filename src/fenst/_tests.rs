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
