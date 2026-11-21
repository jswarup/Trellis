//-- fluxbasics.rs -----------------------------------------------------------------------------------------------------------------------
use crate::flux::{FieldExp, FieldImp, IFluxExportSource, IFluxImportSink, IFluxImportSource};
use crate::silo::{Arr, Buff, MutArr, Stash};

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
        impl $crate::flux::IObjectImport for &mut $struct_name {
            fn Field( &mut self, key: &str)
                -> Result< $crate::flux::FieldImp< '_>, $crate::flux::FluxError>
            {
                $(
                    if key == stringify!( $field) {
                        return Ok( $crate::flux::FieldImp::FluxSource( &mut self.$field));
                    }
                )*
                return Err( $crate::flux::FluxError::TypeMismatch);
            }
        }
        impl $crate::flux::IFluxImportSource for $struct_name {
            fn FetchFieldImp< 'a>( &'a mut self, field: &mut $crate::flux::FieldImp< 'a>)
            {
                *field = $crate::flux::FieldImp::Obj( Box::new( self));
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
        impl $crate::flux::IObjectImport for &mut $struct_name {
            fn Field( &mut self, key: &str)
                -> Result< $crate::flux::FieldImp< '_>, $crate::flux::FluxError>
            {
                if key == "Type" {
                    return Ok( $crate::flux::FieldImp::ExpectedType( $type_name));
                }
                $(
                    if key == stringify!( $field) {
                        return Ok( $crate::flux::FieldImp::FluxSource( &mut self.$field));
                    }
                )*
                return Err( $crate::flux::FluxError::TypeMismatch);
            }
        }
        impl $crate::flux::IFluxImportSource for $struct_name {
            fn FetchFieldImp< 'a>( &'a mut self, field: &mut $crate::flux::FieldImp< 'a>)
            {
                *field = $crate::flux::FieldImp::Obj( Box::new( self));
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
// Borrowed strings cannot accept parser-owned or shorter-lived string data.
impl IFluxImportSink for &str
{
    fn FromFieldImp(&mut self, _field: FieldImp) -> bool { return false; }
}

impl IFluxImportSource for &str
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::FluxSink(self);
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
struct USegImport<'a>
{
    _Target: &'a mut crate::silo::USeg,
    _First:  u64,
    _Last:   u64,
    _Seen:   u8,
}

impl crate::flux::IObjectImport for USegImport<'_>
{
    fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, crate::flux::FluxError>
    {
        let bit = match key {
            "_First" => 1,
            "_Last" => 2,
            _ => return Err(crate::flux::FluxError::TypeMismatch),
        };
        if self._Seen & bit != 0 {
            return Err(crate::flux::FluxError::TypeMismatch);
        }
        self._Seen |= bit;
        return Ok(FieldImp::U64(if bit == 1 {
                                    &mut self._First
                                } else {
                                    &mut self._Last
                                }));
    }

    fn Commit(self: Box<Self>) -> Result<(), crate::flux::FluxError>
    {
        if self._Seen != 3 {
            return Err(crate::flux::FluxError::TypeMismatch);
        }
        let first = u32::try_from(self._First).map_err(|_| crate::flux::FluxError::Overflow)?;
        let last = u32::try_from(self._Last).map_err(|_| crate::flux::FluxError::Overflow)?;
        if first > last && !(first == 1 && last == 0) {
            return Err(crate::flux::FluxError::TypeMismatch);
        }
        *self._Target = crate::silo::USeg::New(first, last);
        return Ok(());
    }
}

impl IFluxImportSource for crate::silo::USeg
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::Obj(Box::new(USegImport { _Target: self,
                                                     _First:  0,
                                                     _Last:   0,
                                                     _Seen:   0, }));
    }
}
impl crate::flux::IFluxImportSink for crate::silo::useg::USeg {
    fn	FromFieldImp( &mut self, field: crate::flux::FieldImp) -> bool
    {
        if let  	crate::flux::FieldImp::Null = field {
            *self = crate::silo::useg::USeg::Empty();
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
// Import requires an exclusive view; Arr remains read-only.
struct ArrayImport<'a, C>
{
    _Target: &'a mut C,
    _Index:  u32,
}

impl<T: IFluxImportSource> crate::flux::IArrayImport for ArrayImport<'_, MutArr<'_, T>>
{
    fn Next(&mut self) -> Result<FieldImp<'_>, crate::flux::FluxError>
    {
        let index = self._Index;
        let item = self._Target
                       .GetMut(index)
                       .ok_or(crate::flux::FluxError::Overflow)?;
        self._Index += 1;
        return Ok(FieldImp::FluxSource(item));
    }
}

impl<T: IFluxImportSource> IFluxImportSource for MutArr<'_, T>
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::Arr(Box::new(ArrayImport { _Target: self,
                                                      _Index:  0, }));
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// Buff: growable heap array
impl<T> IFluxExportSource for Buff<T> where T: IFluxExportSource
{
    fn FetchFieldExp<'b>(&'b self, field: &mut FieldExp<'b>)
    {
        let  	mut idx = 0u32;
        let buff = self;
        *field = FieldExp::Arr( Box::new( move |item| {
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
impl<T: IFluxImportSource> crate::flux::IArrayImport for ArrayImport<'_, Buff<T>>
{
    fn Next(&mut self) -> Result<FieldImp<'_>, crate::flux::FluxError>
    {
        let index = self._Index;
        if index >= self._Target.Size() {
            return Err(crate::flux::FluxError::Overflow);
        }
        self._Index += 1;
        return Ok(FieldImp::FluxSource(&mut self._Target[index]));
    }
}

impl<T: IFluxImportSource> IFluxImportSource for Buff<T>
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::Arr(Box::new(ArrayImport { _Target: self,
                                                      _Index:  0, }));
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
impl<T> IFluxExportSource for crate::silo::Stash<T> where T: IFluxExportSource
{
    fn FetchFieldExp<'b>(&'b self, field: &mut FieldExp<'b>)
    {
        let  	mut idx = 0u32;
        let stash = self;
        *field = FieldExp::Arr( Box::new( move |item| {
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
impl<T: IFluxImportSource + Default> crate::flux::IArrayImport for ArrayImport<'_, Stash<T>>
{
    fn Next(&mut self) -> Result<FieldImp<'_>, crate::flux::FluxError>
    {
        let index = self._Index;
        if index >= self._Target.Size() {
            self._Target.Push(T::default());
        }
        self._Index += 1;
        return Ok(FieldImp::FluxSource(&mut self._Target[index]));
    }
}

impl<T: IFluxImportSource + Default> IFluxImportSource for Stash<T>
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::Arr(Box::new(ArrayImport { _Target: self,
                                                      _Index:  0, }));
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
