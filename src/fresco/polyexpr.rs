// polyexpr.rs -----------------------------------------------------------------------------------
use	crate::flux::{
    fluximport::{ ObjImp, ObjValidate },
    FieldImp, IFluxImportSource,
};
use	crate::fresco::BaseExpr;
use	crate::silo::{ Buff, Stash };
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct PolyExpr
{
    _Children: Buff< u32>,
    _CoSize:   u32,
}

//-------------------------------------------------------------------------------------------------

impl PolyExpr
{
    pub fn	New() -> Self
    {
        Self {
            _Children: Buff::New(),
            _CoSize:   0,
        }
    }

    pub fn	FromParts( coSize: u32, children: Buff< u32>) -> Self
    {
        assert!( coSize <= children.Cap(), "Common child count exceeds child count");
        Self {
            _Children: children,
            _CoSize:   coSize,
        }
    }

    pub fn	SizeChild( &self) -> u32
    { self._Children.Cap() }

    pub fn	Child( &self, index: u32) -> u32
    { self._Children[index] }

    pub fn	CommonSize( &self) -> u32
    { self._CoSize }

    pub fn	Children( &self) -> &Buff< u32>
    { &self._Children }

    pub fn	IntoChildren( self) -> Buff< u32>
    { self._Children }

    pub fn	IsFlip( &self, index: u32) -> bool
    { index >= self._CoSize }
}

//-------------------------------------------------------------------------------------------------

impl Default for PolyExpr
{
    fn	default() -> Self
    { Self::New() }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for PolyExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxExportSource!( PolyExpr, _Children, _CoSize);

//-------------------------------------------------------------------------------------------------

impl IFluxImportSource for PolyExpr
{
    fn	FetchFieldImp< 'a>( &'a mut self, field: &mut FieldImp< 'a>)
    {
        let  	mut children_stash = Box::new( Stash::< u32>::New());
        let  	stash_ptr: *mut Stash< u32> = &mut *children_stash;
        let  	target_ptr = self as *mut Self;

        let  	cb: ObjImp< 'a> = Box::new( move |key, item| {
            let  	target = unsafe { &mut *target_ptr };
            match key
            {
                "_Children" => {
                    let  	stash = unsafe { &mut *stash_ptr };
                    stash.FetchFieldImp( item);
                    true
                }
                "_CoSize" => {
                    target._CoSize.FetchFieldImp( item);
                    true
                }
                _ => false,
            }
        });

        let  	validate: ObjValidate< 'a> = Box::new( move || {
            let  	target = unsafe { &mut *target_ptr };
            target._Children = Buff::FromArr( children_stash.Arr());
            true
        });

        *field = FieldImp::ObjValidated( cb, validate);
    }
}

//-------------------------------------------------------------------------------------------------
