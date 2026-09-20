//-- fluximport.rs -----------------------------------------------------------------------------------------------------------------------
use	std::fmt;
type ObjImp< 'a> = Box< dyn FnMut( &str, &mut FieldImp< 'a>) -> bool + 'a>;

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

#[derive( Default)]
pub enum FieldImp< 'a> {
    #[default]
    Null,
    Str( &'a mut &'a str),
    String( &'a mut String),
    U64( &'a mut u64),
    F64( &'a mut f64),
    Bool( &'a mut bool),
    Arr( Box< dyn FnMut( &mut FieldImp< 'a>) -> bool + 'a>),
    Obj( ObjImp< 'a>),
    FluxSink( &'a mut dyn IFluxImportSink),
    FluxSource( &'a mut dyn IFluxImportSource),
    ExpectedType( &'static str),
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

impl< 'a> FieldImp<'a>
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
            _ => Err( FluxError::TypeMismatch),
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

impl< T: IFluxImportSource + ?Sized> IFluxImportSource for &mut T
{
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        ( **self).FetchFieldImp( field);
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
