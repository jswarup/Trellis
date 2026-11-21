//-- jsonoutstrm.rs -------------------------------------------------------------------------------------------------------------------
use	crate::flux::fluxexport::{ FieldExp, IFluxExportSink };
use	std::{ fmt, mem::swap };

//---------------------------------------------------------------------------------------------------------------------------------

pub struct JsonOutStream< W: fmt::Write>
{
    _OStr: W,
    _Depth: u32,
    _EntryFlg: bool,
    _MultiLineFlg: bool,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< W: fmt::Write> JsonOutStream< W>
{
    pub fn	New( ostr: W, multiLineFlg: bool) -> Self
    {
        Self {
            _OStr: ostr,
            _Depth: 0,
            _EntryFlg: false,
            _MultiLineFlg: multiLineFlg,
        }
    }
    fn	WriteJsonStr( &mut self, value: &str)
    {
        let  	_ = write!( self._OStr, "\"");
        value.chars().for_each( |ch| match ch {
            '\"' => {
                let  	_ = write!( self._OStr, "\\\"");
            }
            '\\' => {
                let  	_ = write!( self._OStr, "\\\\");
            }
            '\u{08}' => {
                let  	_ = write!( self._OStr, "\\b");
            }
            '\u{0C}' => {
                let  	_ = write!( self._OStr, "\\f");
            }
            '\n' => {
                let  	_ = write!( self._OStr, "\\n");
            }
            '\r' => {
                let  	_ = write!( self._OStr, "\\r");
            }
            '\t' => {
                let  	_ = write!( self._OStr, "\\t");
            }
            ch if ch.is_control() => {
                let  	_ = write!( self._OStr, "\\u{:04X}", ch as u32);
            }
            _ => {
                let  	_ = write!( self._OStr, "{}", ch);
            }
        });
        let  	_ = write!( self._OStr, "\"");
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    fn	LineFeed( &mut self) -> fmt::Result
    {
        if self._EntryFlg {
            write!( self._OStr, ",")?;
        }
        if self._MultiLineFlg {
            writeln!( self._OStr)?;
            crate::silo::USeg::FromLen( self._Depth * 2).Traverse( |_| {
                let  	_ = write!( self._OStr, " ");
            });
        } else {
            write!( self._OStr, " ")?;
        }
        Ok( ())
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	KeyField( &mut self, key: &str, value: FieldExp< '_>) -> bool {
        let  	_ = self.LineFeed();
        self._EntryFlg = true;
        if !key.is_empty() {
            self.WriteJsonStr( key);
            let  	_ = write!( self._OStr, ": ");
        }
        self.DispatchFieldExp( value);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< W: fmt::Write> IFluxExportSink for JsonOutStream< W> {
    fn	DispatchFieldExp( &mut self, field: FieldExp)
    {
        match field {
            FieldExp::Str( s) => {
                self.WriteJsonStr( s);
            }
            FieldExp::String( s) => {
                self.WriteJsonStr( &s);
            }
            FieldExp::U64( n) => {
                let  	_ = write!( self._OStr, "{}", n);
            }
            FieldExp::F64( f) => {
                if f.is_nan() || f.is_infinite() {
                    let  	_ = write!( self._OStr, "null");
                } else {
                    let  	_ = write!( self._OStr, "{}", f);
                }
            }
            FieldExp::Bool( b) => {
                let  	_ = write!( self._OStr, "{}", if b { "true" } else { "false" });
            }
            FieldExp::Null => {
                let  	_ = write!( self._OStr, "null");
            }
            FieldExp::Arr( mut arrFunc) => {
                let  	_ = write!( self._OStr, "[");
                let  	mut isFirst = true;
                let  	mut item = FieldExp::Null;
                while arrFunc( &mut item) {
                    if !isFirst {
                        let  	_ = write!( self._OStr, ", ");
                    }
                    let  	mut nextItem = FieldExp::Null;
                    swap( &mut item, &mut nextItem);
                    self.DispatchFieldExp( nextItem);
                    isFirst = false;
                }
                let  	_ = write!( self._OStr, "]");
            }
            FieldExp::Obj( mut objFunc) => {
                let  	_ = write!( self._OStr, "{{");
                self._Depth += 1;
                self._EntryFlg = false;
                let  	mut key = String::new();
                let  	mut item = FieldExp::Null;
                while objFunc( &mut key, &mut item) {
                    let  	mut nextItem = FieldExp::Null;
                    swap( &mut item, &mut nextItem);
                    self.KeyField( &key, nextItem);
                    key.clear();
                }
                if self._Depth > 0 {
                    self._Depth -= 1;
                }
                self._EntryFlg = false;
                let  	_ = self.LineFeed();
                self._EntryFlg = true;
                let  	_ = write!( self._OStr, "}}");
            }
            FieldExp::FluxSource( f) => {
                let  	mut field = FieldExp::Null;
                f.FetchFieldExp( &mut field);
                self.DispatchFieldExp( field);
            }
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
