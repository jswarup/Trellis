//-- fluximport.rs -----------------------------------------------------------------------------------------------------------------------
use	std::fmt;
/// Child descriptors borrow their importer until that child has been consumed.
///
/// Fields must be consumed before looking up another field or committing:
///
/// ```
/// use trellis::flux::{FieldImp, IFluxImportSource};
/// use trellis::silo::USeg;
/// let mut target = USeg::Empty();
/// {
///     let mut field = FieldImp::Null;
///     target.FetchFieldImp(&mut field);
///     let FieldImp::Obj(mut importer) = field else { panic!("object expected") };
///     importer.Field("_First").unwrap().TryPostU64(10).unwrap();
///     importer.Field("_Last").unwrap().TryPostU64(20).unwrap();
///     importer.Commit().unwrap();
/// }
/// assert_eq!(target, USeg::New(10, 20));
/// ```
///
/// A child cannot outlive the importer that owns its staging storage:
///
/// ```compile_fail,E0505
/// use trellis::flux::{FieldImp, IFluxImportSource};
/// use trellis::silo::USeg;
/// let mut target = USeg::Empty();
/// let mut field = FieldImp::Null;
/// target.FetchFieldImp(&mut field);
/// let FieldImp::Obj(mut importer) = field else { panic!("object expected") };
/// let child = importer.Field("_First").unwrap();
/// drop(importer);
/// child.PostU64(10);
/// ```
///
/// Commit cannot invalidate an outstanding child:
///
/// ```compile_fail,E0505
/// use trellis::flux::{FieldImp, IFluxImportSource};
/// use trellis::silo::USeg;
/// let mut target = USeg::Empty();
/// let mut field = FieldImp::Null;
/// target.FetchFieldImp(&mut field);
/// let FieldImp::Obj(mut importer) = field else { panic!("object expected") };
/// let child = importer.Field("_First").unwrap();
/// importer.Commit().unwrap();
/// child.PostU64(10);
/// ```
///
/// A second lookup cannot alias an outstanding mutable field:
///
/// ```compile_fail,E0499
/// use trellis::flux::{FieldImp, IFluxImportSource};
/// use trellis::silo::USeg;
/// let mut target = USeg::Empty();
/// let mut field = FieldImp::Null;
/// target.FetchFieldImp(&mut field);
/// let FieldImp::Obj(mut importer) = field else { panic!("object expected") };
/// let first = importer.Field("_First").unwrap();
/// let second = importer.Field("_Last").unwrap();
/// first.PostU64(10);
/// second.PostU64(20);
/// ```
pub trait IObjectImport
{
    fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, FluxError>;

    fn Commit(self: Box<Self>) -> Result<(), FluxError> { return Ok(()); }
}

//-------------------------------------------------------------------------------------------------

/// An array cannot grow while one of its elements remains borrowed:
///
/// ```compile_fail,E0499
/// use trellis::flux::{FieldImp, IFluxImportSource};
/// use trellis::silo::Stash;
/// let mut target = Stash::<u64>::New();
/// let mut field = FieldImp::Null;
/// target.FetchFieldImp(&mut field);
/// let FieldImp::Arr(mut importer) = field else { panic!("array expected") };
/// let first = importer.Next().unwrap();
/// let second = importer.Next().unwrap();
/// first.TryPostU64(10).unwrap();
/// second.TryPostU64(20).unwrap();
/// ```
pub trait IArrayImport
{
    fn Next(&mut self) -> Result<FieldImp<'_>, FluxError>;
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluxError {
    InvalidSyntax,
    TypeMismatch,
    UnexpectedEof,
    Overflow,
}
impl fmt::Display for FluxError {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        match self {
            FluxError::InvalidSyntax => write!( f, "Invalid syntax"),
            FluxError::TypeMismatch => write!( f, "Type mismatch"),
            FluxError::UnexpectedEof => write!( f, "Unexpected EOF"),
            FluxError::Overflow => write!( f, "Numeric overflow"),
        }
    }
}
impl std::error::Error for FluxError {}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Default)]
pub enum FieldImp<'a>
{
    #[default]
    Null,
    Str(&'a mut &'a str),
    String(&'a mut String),
    U64(&'a mut u64),
    F64(&'a mut f64),
    Bool(&'a mut bool),
    Arr(Box<dyn IArrayImport + 'a>),
    Obj(Box<dyn IObjectImport + 'a>),
    FluxSink(&'a mut dyn IFluxImportSink),
    BoxedSink(Box<dyn IFluxImportSink + 'a>),
    FluxSource(&'a mut dyn IFluxImportSource),
    ExpectedType(&'static str),
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IFluxImportSink {
    fn	FromFieldImp( &mut self, field: FieldImp) -> bool
    {
        self.TryFromFieldImp( field).is_ok()
    }
    fn	TryFromFieldImp( &mut self, field: FieldImp) -> Result< (), FluxError>
    {
        if self.FromFieldImp( field) {
            Ok( ())
        } else {
            Err( FluxError::TypeMismatch)
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> FieldImp<'a>
{
    pub fn	Resolve( &mut self)
    {
        let  	mut temp = FieldImp::Null;
        std::mem::swap( self, &mut temp);
        if let  	FieldImp::FluxSource( src) = temp {
            src.FetchFieldImp( self);
        } else {
            *self = temp;
        }
    }
    pub fn Commit(self) -> Result<(), FluxError>
    {
        match self {
            Self::Obj( importer) => importer.Commit(),
            _ => Ok( ()),
        }
    }
    pub fn	TryPostU64( mut self, val: u64) -> Result< (), FluxError>
    {
        self.Resolve();
        match self {
            FieldImp::U64( dst) => {
                *dst = val;
                Ok( ())
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::U64( &mut temp))
            }
            FieldImp::BoxedSink( mut flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::U64( &mut temp))
            }
            _ => Err( FluxError::TypeMismatch),
        }
    }
    pub fn	TryPostF64( mut self, val: f64) -> Result< (), FluxError>
    {
        self.Resolve();
        match self {
            FieldImp::F64( dst) => {
                *dst = val;
                Ok( ())
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::F64( &mut temp))
            }
            FieldImp::BoxedSink( mut flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::F64( &mut temp))
            }
            _ => Err( FluxError::TypeMismatch),
        }
    }
    pub fn TryPostString(mut self, val: String) -> Result<(), FluxError>
    {
        self.Resolve();
        match self {
            FieldImp::String(dst) => {
                *dst = val;
                Ok(())
            }
            FieldImp::Str(_) => Err(FluxError::TypeMismatch),
            FieldImp::FluxSink(flx) => {
                let mut temp = val;
                flx.TryFromFieldImp(FieldImp::String(&mut temp))
            }
            FieldImp::BoxedSink(mut flx) => {
                let mut temp = val;
                flx.TryFromFieldImp(FieldImp::String(&mut temp))
            }
            _ => Err(FluxError::TypeMismatch),
        }
    }
    pub fn	TryPostStr( mut self, val: &'a str) -> Result<(), FluxError> {
        self.Resolve();
        match self {
            FieldImp::Str( dst) => {
                *dst = val;
                Ok( ())
            }
            FieldImp::String( dst) => {
                *dst = val.to_string();
                Ok( ())
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::Str( &mut temp))
            }
            FieldImp::BoxedSink( mut flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::Str( &mut temp))
            }
            _ => Err( FluxError::TypeMismatch),
        }
    }
    pub fn	TryPostBool( mut self, val: bool) -> Result< (), FluxError>
    {
        self.Resolve();
        match self {
            FieldImp::Bool( dst) => {
                *dst = val;
                Ok( ())
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::Bool( &mut temp))
            }
            FieldImp::BoxedSink( mut flx) => {
                let  	mut temp = val;
                flx.TryFromFieldImp( FieldImp::Bool( &mut temp))
            }
            _ => Err( FluxError::TypeMismatch),
        }
    }
    pub fn	TryPostParsed( mut self, s: &'a str) -> Result<(), FluxError> {
        self.Resolve();
        if let  	Ok( v) = s.parse::< u64>() {
            self.TryPostU64( v)
        } else if let  	Ok( v) = s.parse::< f64>() {
            self.TryPostF64( v)
        } else if let  	Ok( v) = s.parse::< bool>() {
            self.TryPostBool( v)
        } else {
            self.TryPostStr( s)
        }
    }
    pub fn	PostU64( self, val: u64) -> bool
    {
        self.TryPostU64( val).is_ok()
    }
    pub fn	PostF64( self, val: f64) -> bool
    {
        self.TryPostF64( val).is_ok()
    }
    pub fn	PostStr( self, val: &'a str) -> bool {
        self.TryPostStr( val).is_ok()
    }
    pub fn	PostBool( self, val: bool) -> bool
    {
        self.TryPostBool( val).is_ok()
    }
    pub fn	PostParsed( self, s: &'a str) -> bool {
        self.TryPostParsed( s).is_ok()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IFluxImportSource {
    fn	FetchFieldImp< 'a>(&'a mut self, _field: &mut FieldImp< 'a>) {}
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< T: IFluxImportSource + ?Sized> IFluxImportSource for &mut T {
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        ( **self).FetchFieldImp( field);
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
