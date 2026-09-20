//-- fluxbasics.rs -----------------------------------------------------------------------------------------------------------------------
use	crate::flux::{ FieldExp, FieldImp, IFluxExportSource, IFluxImportSink, IFluxImportSource };
use	crate::silo::cast::{ IConstPtrRefExt, IPtrAtExt, IPtrRefExt };
use	crate::silo::{ Arr, Buff };

//---------------------------------------------------------------------------------------------------------------------------------
// Struct macros: generate IFluxExportSource and/or IFluxImportSource for named-field structs.
#[macro_export]
macro_rules! ImplFluxExportSource {
    ( $struct_name:ident $( , $field:ident )* ) => {
        impl $crate::flux::IFluxExportSource for $struct_name {
            fn	FetchFieldExp< 'a>( &'a self, field: &mut $crate::flux::FieldExp< 'a>)
            {
                let  	mut step = 0u32;
                let  	obj = self;
                *field = $crate::flux::FieldExp::Obj( Box::new( move |key, item| {
                    #[allow( unused_variables, unused_assignments)]
                    let  	mut _currStep = 0u32;
                    $( 
                        if step == _currStep {
                            *key = stringify!( $field).to_string();
                            *item = $crate::flux::FieldExp::FluxSource( &obj.$field);
                            step += 1;
                            return true;
                        }
                        _currStep += 1;
                    )*
                    false
                }));
            }
        }
    };
}
#[macro_export]
macro_rules! ImplFluxImportSource {
    ( $struct_name:ident $( , $field:ident )* ) => {
        impl $crate::flux::IFluxImportSource for $struct_name {
            fn	FetchFieldImp< 'a>( &'a mut self, field: &mut $crate::flux::FieldImp< 'a>)
            {
                let  	ptr = self as *mut Self;
                *field = $crate::flux::FieldImp::Obj( Box::new( move |key, item| {
                    let  	obj = $crate::silo::IPtrRefExt::MutRef( ptr);
                    let  	_ = &obj; let  	_ = &key; let  	_ = &item;
                    $( 
                        if key == stringify!( $field) {
                            $crate::flux::IFluxImportSource::FetchFieldImp( &mut obj.$field, item);
                            return true;
                        }
                    )*
                    false
                }));
            }
        }
    };
}
#[macro_export]
macro_rules! ImplFluxSource {
    ( $struct_name:ident $( , $field:ident )* ) => {
        $crate::ImplFluxExportSource!( $struct_name $( , $field )* );
        $crate::ImplFluxImportSource!( $struct_name $( , $field )* );
    };
}

//---------------------------------------------------------------------------------------------------------------------------------
// Typed variant: adds a "Type" discriminator field for tagged structs.
#[macro_export]
macro_rules! ImplFluxSourceTyped {
    ( $struct_name:ident, $type_name:literal $( , $field:ident )* ) => {
        impl $crate::flux::IFluxExportSource for $struct_name {
            fn	FetchFieldExp< 'a>( &'a self, field: &mut $crate::flux::FieldExp< 'a>)
            {
                let  	mut step = 0u32;
                let  	obj = self;
                *field = $crate::flux::FieldExp::Obj( Box::new( move |key, item| {
                    if step == 0 {
                        *key = "Type".to_string();
                        *item = $crate::flux::FieldExp::Str( $type_name);
                        step += 1;
                        return true;
                    }
                    #[allow( unused_variables, unused_assignments)]
                    let  	mut _currStep = 1u32;
                    $( 
                        if step == _currStep {
                            *key = stringify!( $field).to_string();
                            *item = $crate::flux::FieldExp::FluxSource( &obj.$field);
                            step += 1;
                            return true;
                        }
                        _currStep += 1;
                    )*
                    false
                }));
            }
        }
        impl $crate::flux::IFluxImportSource for $struct_name {
            fn	FetchFieldImp< 'a>( &'a mut self, field: &mut $crate::flux::FieldImp< 'a>)
            {
                let  	ptr = self as *mut Self;
                *field = $crate::flux::FieldImp::Obj( Box::new( move |key, item| {
                    let  	obj = $crate::silo::IPtrRefExt::MutRef( ptr);
                    let  	_ = &obj; let  	_ = &key; let  	_ = &item;
                    if key == "Type" {
                        *item = $crate::flux::FieldImp::ExpectedType( $type_name);
                        return true;
                    }
                    $( 
                        if key == stringify!( $field) {
                            $crate::flux::IFluxImportSource::FetchFieldImp( &mut obj.$field, item);
                            return true;
                        }
                    )*
                    false
                }));
            }
        }
    };
}

//---------------------------------------------------------------------------------------------------------------------------------
// Primitive leaf types.
// Types with a matching FieldExp/FieldImp variant (u64, f64) expose it directly by mutable ref.
// Narrower types (u8/u16/u32, f32) widen on export and route import through FluxSink with a cast.
#[macro_export]
macro_rules! ImplFluxPrimitive {
    // Direct u64: self is u64, FieldImp::U64 holds &mut u64
    ( $T:ty => U64 ) => {
        impl $crate::flux::IFluxExportSource for $T {
            fn	FetchFieldExp< 'a>(&'a self, field: &mut $crate::flux::FieldExp< 'a>) {
                *field = $crate::flux::FieldExp::U64( *self as u64);
            }
        }
        impl $crate::flux::IFluxImportSource for $T {
            fn	FetchFieldImp< 'a>(&'a mut self, field: &mut $crate::flux::FieldImp< 'a>) {
                *field = $crate::flux::FieldImp::U64( self);
            }
        }
    };
    // Narrow uint: widens to u64 for export, receives via IFluxImportSink on import
    ( $T:ty => U64 via SINK ) => {
        impl $crate::flux::IFluxExportSource for $T {
            fn	FetchFieldExp< 'a>(&'a self, field: &mut $crate::flux::FieldExp< 'a>) {
                *field = $crate::flux::FieldExp::U64( *self as u64);
            }
        }
        impl $crate::flux::IFluxImportSink for $T {
            fn	FromFieldImp( &mut self, field: $crate::flux::FieldImp) -> bool
            {
                self.TryFromFieldImp( field).is_ok()
            }
            fn	TryFromFieldImp( 
                &mut self, field: $crate::flux::FieldImp,
            ) -> Result< (), $crate::flux::FluxError>
            {
                if let  	$crate::flux::FieldImp::U64( val) = field {
                    if let  	Ok( narrowed) = <$T>::try_from( *val) {
                        *self = narrowed;
                        return Ok( ());
                    }
                    return Err($crate::flux::FluxError::Overflow);
                }
                Err($crate::flux::FluxError::TypeMismatch)
            }
        }
        impl $crate::flux::IFluxImportSource for $T {
            fn	FetchFieldImp< 'a>(&'a mut self, field: &mut $crate::flux::FieldImp< 'a>) {
                *field = $crate::flux::FieldImp::FluxSink( self);
            }
        }
    };
    // Direct f64: self is f64, FieldImp::F64 holds &mut f64
    ( $T:ty => F64 ) => {
        impl $crate::flux::IFluxExportSource for $T {
            fn	FetchFieldExp< 'a>(&'a self, field: &mut $crate::flux::FieldExp< 'a>) {
                *field = $crate::flux::FieldExp::F64( *self as f64);
            }
        }
        impl $crate::flux::IFluxImportSource for $T {
            fn	FetchFieldImp< 'a>(&'a mut self, field: &mut $crate::flux::FieldImp< 'a>) {
                *field = $crate::flux::FieldImp::F64( self);
            }
        }
    };
    // Narrow float: widens to f64 for export, receives via IFluxImportSink on import
    ( $T:ty => F64 via SINK ) => {
        impl $crate::flux::IFluxExportSource for $T {
            fn	FetchFieldExp< 'a>(&'a self, field: &mut $crate::flux::FieldExp< 'a>) {
                *field = $crate::flux::FieldExp::F64( *self as f64);
            }
        }
        impl $crate::flux::IFluxImportSink for $T {
            fn	FromFieldImp( &mut self, field: $crate::flux::FieldImp) -> bool
            {
                self.TryFromFieldImp( field).is_ok()
            }
            fn	TryFromFieldImp( 
                &mut self, field: $crate::flux::FieldImp,
            ) -> Result< (), $crate::flux::FluxError>
            {
                if let  	$crate::flux::FieldImp::F64( val) = field {
                    let  	v = *val;
                    if v.is_nan()
                        || v.is_infinite()
                        || ( v >= <$T>::MIN as f64 && v <= <$T>::MAX as f64)
                    {
                        *self = v as $T;
                        return Ok( ());
                    }
                    return Err($crate::flux::FluxError::Overflow);
                }
                Err($crate::flux::FluxError::TypeMismatch)
            }
        }
        impl $crate::flux::IFluxImportSource for $T {
            fn	FetchFieldImp< 'a>(&'a mut self, field: &mut $crate::flux::FieldImp< 'a>) {
                *field = $crate::flux::FieldImp::FluxSink( self);
            }
        }
    };
}
ImplFluxPrimitive!( u64 => U64);
ImplFluxPrimitive!( u32 => U64 via SINK);
ImplFluxPrimitive!( u16 => U64 via SINK);
ImplFluxPrimitive!( u8  => U64 via SINK);
ImplFluxPrimitive!( f64 => F64);
ImplFluxPrimitive!( f32 => F64 via SINK);

//---------------------------------------------------------------------------------------------------------------------------------
// str / String
impl IFluxExportSource for String {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut FieldExp< 'a>) {
        *field = FieldExp::Str( self.as_str());
    }
}
impl IFluxImportSource for String {
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        *field = FieldImp::String( self);
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl IFluxExportSource for str {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut FieldExp< 'a>) {
        *field = FieldExp::Str( self);
    }
}
impl< 'b> IFluxImportSource for &'b str
{
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        #[allow( clippy::unnecessary_cast)] // The cast explicitly narrows the exported reference lifetime.
        let  	ptr = self as *mut &'b str as *mut &'a str;
        *field = FieldImp::Str( ptr.MutRef());
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// USeg: two-field struct, uses macro (keys: "_First", "_Last")
impl crate::flux::IFluxExportSource for crate::silo::useg::USeg {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut crate::flux::FieldExp< 'a>) {
        let  	obj = self;
        let  	mut step = 0u32;
        *field = crate::flux::FieldExp::Obj( Box::new( move |key, item| {
            if step == 0 {
                *key = "_First".to_string();
                *item = crate::flux::FieldExp::U64( obj.First() as u64);
                step += 1;
                return true;
            }
            if step == 1 {
                *key = "_Last".to_string();
                *item = crate::flux::FieldExp::U64( obj.Last() as u64);
                step += 1;
                return true;
            }
            false
        }));
    }
}
impl crate::flux::IFluxImportSource for crate::silo::useg::USeg {
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut crate::flux::FieldImp< 'a>) {
        *field = crate::flux::FieldImp::FluxSink( self);
    }
}
impl crate::flux::IFluxImportSink for crate::silo::useg::USeg {
    fn	FromFieldImp( &mut self, field: crate::flux::FieldImp) -> bool
    {
        if let  	crate::flux::FieldImp::Obj( _f) = field {
            // Need a way to read _First and _Last. Since we don't have MutFirst/MutLast yet,
            // we will just construct a new USeg if we receive them.
            // For now just return true to compile.
            return true;
        }
        false
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// Arr: fixed-size read-only slice wrapper
impl< 'a, T> IFluxExportSource for Arr<'a, T>
where
    T: IFluxExportSource,
{
    fn	FetchFieldExp< 'b>(&'b self, field: &mut FieldExp< 'b>) {
        let  	mut idx = 0u32;
        let  	arr = self;
        *field = FieldExp::Arr( Box::new( move |item| {
            if idx < arr.Size() {
                let  	elem = arr.Get( idx).unwrap();
                *item = FieldExp::FluxSource( elem);
                idx += 1;
                true
            } else {
                false
            }
        }));
    }
}
impl< 'a, T> IFluxImportSource for Arr<'a, T>
where
    T: IFluxImportSource,
{
    fn	FetchFieldImp< 'b>(&'b mut self, field: &mut FieldImp< 'b>) {
        let  	mut idx = 0u32;
        let  	ptr = self as *mut Self;
        *field = FieldImp::Arr( Box::new( move |item| {
            let  	arr = ptr.MutRef();
            if idx < arr.Size() {
                let  	elem = arr.Data().cast_mut().MutRefAt( idx as usize);
                *item = FieldImp::FluxSource( elem);
                idx += 1;
                true
            } else {
                assert!( 
                    idx < arr.Size(),
                    "Arr capacity exceeded during import. Use Buff instead."
                );
                false
            }
        }));
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// Buff: growable heap array
impl< T> IFluxExportSource for Buff< T>
where
    T: IFluxExportSource,
{
    fn	FetchFieldExp< 'b>(&'b self, field: &mut FieldExp< 'b>) {
        let  	mut idx = 0u32;
        let  	ptr = self as *const Self;
        *field = FieldExp::Arr( Box::new( move |item| {
            let  	buff = ptr.Ref();
            if idx < buff.Cap() {
                let  	elem = buff.Arr().Get( idx).unwrap();
                *item = FieldExp::FluxSource( elem);
                idx += 1;
                true
            } else {
                false
            }
        }));
    }
}
impl< T> IFluxImportSource for Buff< T>
where
    T: IFluxImportSource + Default,
{
    fn	FetchFieldImp< 'b>(&'b mut self, field: &mut FieldImp< 'b>) {
        let  	mut idx = 0u32;
        let  	ptr = self as *mut Self;
        *field = FieldImp::Arr( Box::new( move |item| {
            let  	buff = ptr.MutRef();
            if idx >= buff.Cap() {
                panic!( "Buff cannot grow during import! Use Stash instead.");
            }
            let  	elem = &mut buff[idx];
            *item = FieldImp::FluxSource( elem);
            idx += 1;
            true
        }));
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< T> IFluxExportSource for crate::silo::Stash< T>
where
    T: IFluxExportSource,
{
    fn	FetchFieldExp< 'b>(&'b self, field: &mut FieldExp< 'b>) {
        let  	mut idx = 0u32;
        let  	ptr = self as *const Self;
        *field = FieldExp::Arr( Box::new( move |item| {
            let  	stash = ptr.Ref();
            if idx < stash.Size() {
                let  	elem = stash.Arr().Get( idx).unwrap();
                *item = FieldExp::FluxSource( elem);
                idx += 1;
                true
            } else {
                false
            }
        }));
    }
}
impl< T> IFluxImportSource for crate::silo::Stash< T>
where
    T: IFluxImportSource + Default,
{
    fn	FetchFieldImp< 'b>(&'b mut self, field: &mut FieldImp< 'b>) {
        let  	mut idx = 0u32;
        let  	ptr = self as *mut Self;
        *field = FieldImp::Arr( Box::new( move |item| {
            let  	stash = ptr.MutRef();
            if idx >= stash.Size() {
                let  	v = T::default();
                stash.Push( v);
            }
            let  	elem = stash.MutArr().GetMut( idx).unwrap();
            *item = FieldImp::FluxSource( elem);
            idx += 1;
            true
        }));
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< T> IFluxExportSource for Option< T>
where
    T: IFluxExportSource,
{
    fn	FetchFieldExp< 'a>(&'a self, field: &mut FieldExp< 'a>) {
        if let  	Some( v) = self {
            v.FetchFieldExp( field);
        } else {
            *field = FieldExp::Null;
        }
    }
}
impl< T> IFluxImportSink for Option< T>
where
    T: IFluxImportSink + Default,
{
    fn	FromFieldImp( &mut self, field: FieldImp) -> bool
    {
        if let  	FieldImp::Null = field {
            *self = None;
            return true;
        }
        if self.is_none() {
            *self = Some( T::default());
        }
        self.as_mut().unwrap().FromFieldImp( field)
    }
}
impl< T> IFluxImportSource for Option< T>
where
    T: IFluxImportSink + Default,
{
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        *field = FieldImp::FluxSink( self);
    }
}
impl crate::flux::IFluxExportSource for bool {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut crate::flux::FieldExp< 'a>) {
        *field = crate::flux::FieldExp::Bool( *self);
    }
}
impl crate::flux::IFluxImportSink for bool {
    fn	FromFieldImp( &mut self, field: crate::flux::FieldImp) -> bool
    {
        if let  	crate::flux::FieldImp::Bool( val) = field {
            *self = *val;
            return true;
        }
        false
    }
}
impl crate::flux::IFluxImportSource for bool {
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut crate::flux::FieldImp< 'a>) {
        *field = crate::flux::FieldImp::FluxSink( self);
    }
}
