// file.rs ----------------------------------------------------------------------------------------
//! Bounded native FLX/TFLX storage over the existing Flux export descriptors.
//! Files are decoded into owned staging values; callers validate their schema before publishing.

use crate::flux::{FieldExp, IFluxExportSource};
use crate::silo::{Arr, Buff, IArr, Stash, USeg};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_VALUES: u32 = 2_000_000;
const MAX_DEPTH: u32 = 64;
const MAGIC: &[u8; 8] = b"TRFLX\r\n\x1a";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum FluxValue
{
    Null,
    Bool(bool),
    U64(u64),
    F64(f64),
    String(String),
    Array(Buff<FluxValue>),
    Object(Buff<(String, FluxValue)>),
}

impl FluxValue
{
    pub fn Record(fields: Buff<(&str, Self)>) -> Self
    {
        return Self::Object(Buff::FromDispenser(fields.Size(), |i| {
                                (fields[i].0.to_string(), fields[i].1.clone())
                            }));
    }

    pub fn Get(&self, key: &str) -> Result<&Self, String>
    {
        if let Self::Object(fields) = self {
            let mut index = 0;
            while index < fields.Size() {
                if fields[index].0 == key {
                    return Ok(&fields[index].1);
                }
                index += 1;
            }
        }
        return Err(format!("Missing Flux field: {key}"));
    }

    pub fn Text(&self) -> Result<&str, String>
    {
        if let Self::String(value) = self {
            return Ok(value);
        }
        return Err("Expected a Flux string".into());
    }

    pub fn UInt(&self) -> Result<u64, String>
    {
        if let Self::U64(value) = self {
            return Ok(*value);
        }
        return Err("Expected a Flux u64".into());
    }

    pub fn Real(&self) -> Result<f64, String>
    {
        if let Self::F64(value) = self {
            return Ok(*value);
        }
        return Err("Expected a Flux f64".into());
    }

    pub fn Boolean(&self) -> Result<bool, String>
    {
        if let Self::Bool(value) = self {
            return Ok(*value);
        }
        return Err("Expected a Flux bool".into());
    }

    pub fn Items(&self) -> Result<Arr<'_, Self>, String>
    {
        if let Self::Array(values) = self {
            return Ok(values.Arr());
        }
        return Err("Expected a Flux array".into());
    }

    pub fn FromSource(source: &dyn IFluxExportSource) -> Result<Self, String>
    {
        let mut field = FieldExp::Null;
        source.FetchFieldExp(&mut field);
        return Self::Collect(field, 0, &mut 0);
    }

    fn Collect(field: FieldExp<'_>, depth: u32, count: &mut u32) -> Result<Self, String>
    {
        Self::Budget(depth, count)?;
        return Ok(match field {
            FieldExp::Null => Self::Null,
            FieldExp::Bool(v) => Self::Bool(v),
            FieldExp::U64(v) => Self::U64(v),
            FieldExp::F64(v) => Self::F64(v),
            FieldExp::Str(v) => Self::String(v.to_string()),
            FieldExp::String(v) => Self::String(v),
            FieldExp::FluxSource(source) => {
                let mut next = FieldExp::Null;
                source.FetchFieldExp(&mut next);
                Self::Collect(next, depth + 1, count)?
            }
            FieldExp::Arr(mut next) => {
                let mut values = Stash::New();
                let mut field = FieldExp::Null;
                while next(&mut field) {
                    values.Push(Self::Collect(std::mem::replace(&mut field, FieldExp::Null),
                                              depth + 1,
                                              count)?);
                }
                Self::Array(values.IntoBuff())
            }
            FieldExp::Obj(mut next) => {
                let mut fields = Stash::New();
                let mut keys = BTreeSet::new();
                let mut key = String::new();
                let mut field = FieldExp::Null;
                while next(&mut key, &mut field) {
                    if !keys.insert(key.clone()) {
                        return Err(format!("Duplicate Flux key: {key}"));
                    }
                    let value = Self::Collect(std::mem::replace(&mut field, FieldExp::Null),
                                              depth + 1,
                                              count)?;
                    fields.Push((std::mem::take(&mut key), value));
                }
                Self::Object(fields.IntoBuff())
            }
        });
    }

    fn Budget(depth: u32, count: &mut u32) -> Result<(), String>
    {
        *count += 1;
        if depth > MAX_DEPTH || *count > MAX_VALUES {
            return Err("Flux nesting/value limit exceeded".into());
        }
        return Ok(());
    }
}

impl IFluxExportSource for FluxValue
{
    fn FetchFieldExp<'a>(&'a self, field: &mut FieldExp<'a>)
    {
        *field = match self {
            Self::Null => FieldExp::Null,
            Self::Bool(v) => FieldExp::Bool(*v),
            Self::U64(v) => FieldExp::U64(*v),
            Self::F64(v) => FieldExp::F64(*v),
            Self::String(v) => FieldExp::Str(v),
            Self::Array(values) => {
                let mut index = 0;
                FieldExp::Arr(Box::new(move |item| {
                                  if index == values.Size() {
                                      return false;
                                  }
                                  *item = FieldExp::FluxSource(&values[index]);
                                  index += 1;
                                  return true;
                              }))
            }
            Self::Object(fields) => {
                let mut index = 0;
                FieldExp::Obj(Box::new(move |key, item| {
                                  if index == fields.Size() {
                                      return false;
                                  }
                                  *key = fields[index].0.clone();
                                  *item = FieldExp::FluxSource(&fields[index].1);
                                  index += 1;
                                  return true;
                              }))
            }
        };
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluxFileFormat
{
    Binary,
    Text,
}

impl FluxFileFormat
{
    pub fn FromPath(path: &Path) -> Result<Self, String>
    {
        let extension = path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
        return match extension.as_str() {
            "flx" => Ok(Self::Binary),
            "tflx" => Ok(Self::Text),
            _ => Err("Choose a filename ending in .flx or .tflx".into()),
        };
    }
}

pub struct FluxFile
{
    _Metadata: FluxValue,
    _Payload:  FluxValue,
    _IntegrityVerified: bool,
}

impl FluxFile
{
    pub fn New(schema: &str, version: u64, payload: FluxValue) -> Result<Self, String>
    {
        let metadata = FluxValue::Record(crate::Buff![
            ("Schema", FluxValue::String(schema.into())),
            ("SchemaVersion", FluxValue::U64(version)),
            ("DocumentId", FluxValue::String(Self::NewId()?)),
            ("DocumentRevision", FluxValue::U64(0)),
            ("CreatedUtc", FluxValue::Null),
            ("ModifiedUtc", FluxValue::Null),
            (
                "Producer",
                FluxValue::Record(crate::Buff![
                    ("Name", FluxValue::String("Trellis".into())),
                    (
                        "Version",
                        FluxValue::String(env!("CARGO_PKG_VERSION").into())
                    ),
                ])
            ),
        ]);
        return Self::FromParts(metadata, payload);
    }

    pub fn FromParts(metadata: FluxValue, payload: FluxValue) -> Result<Self, String>
    {
        if metadata.Get("Schema")?.Text()?.is_empty() || metadata.Get("SchemaVersion")?.UInt()? == 0
        {
            return Err("Invalid Flux schema metadata".into());
        }
        let id = metadata.Get("DocumentId")?.Text()?;
        if id.len() != 32
           || !id.bytes()
                 .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
           || id.bytes().all(|b| b == b'0')
        {
            return Err("Invalid Flux document identity".into());
        }
        metadata.Get("DocumentRevision")?.UInt()?;
        Self::ValidateTime(metadata.Get("CreatedUtc")?)?;
        Self::ValidateTime(metadata.Get("ModifiedUtc")?)?;
        let producer = metadata.Get("Producer")?;
        if producer.Get("Name")?.Text()?.is_empty() || producer.Get("Version")?.Text()?.is_empty() {
            return Err("Invalid Flux producer metadata".into());
        }
        if let Ok(extension) = metadata.Get("Extensions") && !matches!(extension, FluxValue::Object(_)) {
            return Err("Flux metadata Extensions must be an object".into());
        }
        return Ok(Self { _Metadata: metadata, _Payload: payload, _IntegrityVerified: false });
    }

    fn ValidateTime(value: &FluxValue) -> Result<(), String>
    {
        if matches!(value, FluxValue::Null) {
            return Ok(());
        }
        let text = value.Text()?;
        let b = text.as_bytes();
        if b.len() != 30
           || b[4] != b'-'
           || b[7] != b'-'
           || b[10] != b'T'
           || b[13] != b':'
           || b[16] != b':'
           || b[19] != b'.'
           || b[29] != b'Z'
        {
            return Err("Invalid UTC Flux timestamp".into());
        }
        let number = |start, end| {
            text.get(start..end)
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or("Invalid UTC digits")
        };
        let year = number(0, 4)?;
        let month = number(5, 7)?;
        let day = number(8, 10)?;
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days = match month {
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        };
        if year == 0
           || day == 0
           || day > days
           || number(11, 13)? > 23
           || number(14, 16)? > 59
           || number(17, 19)? > 59
           || !b[20..29].iter().all(u8::is_ascii_digit)
        {
            return Err("Invalid UTC calendar value".into());
        }
        return Ok(());
    }

    pub fn Metadata(&self) -> &FluxValue { return &self._Metadata; }
    pub fn Payload(&self) -> &FluxValue { return &self._Payload; }

    pub fn IntegrityVerified(&self) -> bool { return self._IntegrityVerified; }

    pub fn Encode(&self, format: FluxFileFormat) -> Result<Buff<u8>, String>
    {
        let metadata = BinaryBytes(&self._Metadata)?;
        if metadata.Size() > 1024 * 1024 {
            return Err("Flux metadata exceeds 1 MiB limit".into());
        }
        let payload = BinaryBytes(&self._Payload)?;
        let mc = Crc32c(metadata.Arr());
        let pc = Crc32c(payload.Arr());
        if format == FluxFileFormat::Text {
            let mut text = String::from("TFLX 1.0\nFeatures u64(0);\nMetadata ");
            TextValue(&self._Metadata, &mut text, 0)?;
            if text.len() > 1024 * 1024 {
                return Err("Flux text metadata exceeds 1 MiB limit".into());
            }
            text.push_str(";\nData ");
            TextValue(&self._Payload, &mut text, 0)?;
            text.push_str(&format!(";\nIntegrity crc32c(0x{mc:08x}, 0x{pc:08x});\nEnd;\n"));
            if text.len() > MAX_BYTES {
                return Err("Flux file exceeds 16 MiB limit".into());
            }
            return Ok(Buff::FromDispenser(text.len() as u32, |i| {
                text.as_bytes()[i as usize]
            }));
        }
        let length = 64 + metadata.Size() + payload.Size();
        if length as usize > MAX_BYTES {
            return Err("Flux file exceeds 16 MiB limit".into());
        }
        let mut header = [0u8; 64];
        header[..8].copy_from_slice(MAGIC);
        header[8..10].copy_from_slice(&1u16.to_le_bytes());
        header[12..16].copy_from_slice(&64u32.to_le_bytes());
        header[24..32].copy_from_slice(&u64::from(length).to_le_bytes());
        header[32..40].copy_from_slice(&u64::from(metadata.Size()).to_le_bytes());
        header[40..48].copy_from_slice(&u64::from(payload.Size()).to_le_bytes());
        header[48..52].copy_from_slice(&mc.to_le_bytes());
        header[52..56].copy_from_slice(&pc.to_le_bytes());
        let hc = Crc32c(Arr::from(&header[..]));
        header[20..24].copy_from_slice(&hc.to_le_bytes());
        return Ok(Buff::FromDispenser(length, |i| {
            if i < 64 {
                return header[i as usize];
            }
            if i < 64 + metadata.Size() {
                return metadata[i - 64];
            }
            return payload[i - 64 - metadata.Size()];
        }));
    }

    pub fn Decode(input: Arr<'_, u8>) -> Result<Self, String>
    {
        let bytes: &[u8] = input.into();
        if bytes.len() > MAX_BYTES {
            return Err("Flux file exceeds 16 MiB limit".into());
        }
        if bytes.starts_with(MAGIC) {
            if bytes.len() < 64 {
                return Err("Truncated Flux header".into());
            }
            let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let wide = |offset| u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let headerLen = word(12) as usize;
            if bytes[8..12] != [1, 0, 0, 0]
               || word(16) != 0
               || headerLen != 64
               || bytes[56..64] != [0; 8]
            {
                return Err("Unsupported Flux version, features, or header".into());
            }
            let mut header = [0u8; 64];
            header.copy_from_slice(&bytes[..64]);
            header[20..24].fill(0);
            if Crc32c(Arr::from(&header[..])) != word(20) {
                return Err("Flux header checksum mismatch".into());
            }
            let ml = wide(32);
            let pl = wide(40);
            if ml == 0
               || pl == 0
               || wide(24) != bytes.len() as u64
               || ml.checked_add(pl).and_then(|n| n.checked_add(64)) != Some(bytes.len() as u64)
               || ml > 1024 * 1024
            {
                return Err("Invalid Flux region lengths".into());
            }
            let split = 64 + ml as usize;
            if Crc32c(Arr::from(&bytes[64..split])) != word(48)
               || Crc32c(Arr::from(&bytes[split..])) != word(52)
            {
                return Err("Flux metadata/payload checksum mismatch".into());
            }
            let metadata = ValueParser::Binary(&bytes[64..split])?;
            let payload = ValueParser::Binary(&bytes[split..])?;
            let mut file = Self::FromParts(metadata, payload)?;
            file._IntegrityVerified = true;
            return Ok(file);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "Flux text must be UTF-8")?;
        let body = text.strip_prefix("TFLX 1.0\r\n")
                       .or_else(|| text.strip_prefix("TFLX 1.0\n"))
                       .ok_or("Unrecognized Flux file preamble")?;
        let mut parser = ValueParser::New(body.as_bytes());
        parser.Token("Features")?;
        if parser.Value(0)?.UInt()? != 0 {
            return Err("Unsupported Flux features".into());
        }
        parser.Token(";")?;
        parser.Token("Metadata")?;
        let metadata = parser.Value(0)?;
        if parser._Pos > 1024 * 1024 {
            return Err("Flux metadata exceeds 1 MiB limit".into());
        }
        parser.Token(";")?;
        parser.Token("Data")?;
        let payload = parser.Value(0)?;
        parser.Token(";")?;
        parser.Token("Integrity")?;
        parser.Space();
        let verified = !parser.Remaining().starts_with(b"none");
        if !verified {
            parser.Token("none")?;
        } else {
            parser.Token("crc32c")?;
            parser.Token("(")?;
            let mc = parser.Hex(8)? as u32;
            parser.Token(",")?;
            let pc = parser.Hex(8)? as u32;
            parser.Token(")")?;
            if Crc32c(BinaryBytes(&metadata)?.Arr()) != mc
               || Crc32c(BinaryBytes(&payload)?.Arr()) != pc
            {
                return Err("Flux text checksum mismatch".into());
            }
        }
        parser.Token(";")?;
        parser.Token("End")?;
        parser.Token(";")?;
        parser.Space();
        if parser._Pos != parser._Bytes.len() {
            return Err(parser.Error("Trailing file content"));
        }
        let mut file = Self::FromParts(metadata, payload)?;
        file._IntegrityVerified = verified;
        return Ok(file);
    }

    pub fn Read(path: &Path) -> Result<Self, String>
    {
        let format = FluxFileFormat::FromPath(path)?;
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        if size > MAX_BYTES as u64 {
            return Err("Flux file exceeds 16 MiB limit".into());
        }
        let mut data = Buff::FromDispenser(size as u32, |_| 0u8);
        file.read_exact(data.MutArr().into())
            .map_err(|e| e.to_string())?;
        let bytes: &[u8] = data.Arr().into();
        if (format == FluxFileFormat::Binary) != bytes.starts_with(MAGIC) {
            return Err("File contents do not match the .flx/.tflx extension".into());
        }
        let mut trailing = [0u8];
        if file.read(&mut trailing).map_err(|e| e.to_string())? != 0 {
            return Err("File changed while reading".into());
        }
        return Self::Decode(data.Arr());
    }

    pub fn Save(&self, path: &Path) -> Result<(), String>
    {
        let format = FluxFileFormat::FromPath(path)?;
        let bytes = self.Encode(format)?;
        let name = path.file_name()
                       .ok_or("Choose a file name")?
                       .to_string_lossy();
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce));
        let mut created = false;
        let result = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new().write(true)
                                                      .create_new(true)
                                                      .open(&temp)?;
            created = true;
            file.write_all(bytes.Arr().into())?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, path)?;
            return Ok(());
        })();
        if result.is_err() && created {
            let _ = std::fs::remove_file(&temp);
        }
        return result.map_err(|e| format!("Could not save {}: {e}", path.display()));
    }

    pub fn NewId() -> Result<String, String>
    {
        let mut bytes = [0u8; 16];
        #[cfg(windows)]
        {
            #[link(name = "bcrypt")]
            unsafe extern "system" {
                fn BCryptGenRandom(algorithm: *mut std::ffi::c_void, buffer: *mut u8, length: u32,
                                   flags: u32)
                                   -> i32;
            }
            // The OS fills exactly the writable 16-byte buffer; no retained pointer.
            if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 {
                return Err("Could not generate a document identity".into());
            }
        }
        #[cfg(not(windows))]
        std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes))
                                           .map_err(|e| e.to_string())?;
        let mut id = String::with_capacity(32);
        USeg::FromLen(16).Traverse(|i| id.push_str(&format!("{:02x}", bytes[i as usize])));
        return Ok(id);
    }
}

//-------------------------------------------------------------------------------------------------

pub fn Crc32c(bytes: Arr<'_, u8>) -> u32
{
    let mut crc = u32::MAX;
    bytes.Traverse(|byte| {
             crc ^= u32::from(*byte);
             USeg::FromLen(8).Traverse(|_| {
                                 crc = (crc >> 1) ^ if crc & 1 != 0 { 0x82f6_3b78 } else { 0 }
                             });
         });
    return !crc;
}

fn BinaryBytes(value: &FluxValue) -> Result<Buff<u8>, String>
{
    fn Varint(mut value: u64, out: &mut Stash<u8>)
    {
        loop {
            let byte = (value & 127) as u8;
            value >>= 7;
            out.Push(byte | if value != 0 { 128 } else { 0 });
            if value == 0 {
                return;
            }
        }
    }
    fn Encode(value: &FluxValue, out: &mut Stash<u8>, depth: u32, count: &mut u32)
              -> Result<(), String>
    {
        FluxValue::Budget(depth, count)?;
        match value {
            FluxValue::Null => out.Push(0),
            FluxValue::Bool(false) => out.Push(1),
            FluxValue::Bool(true) => out.Push(2),
            FluxValue::U64(v) => {
                out.Push(3);
                Varint(*v, out);
            }
            FluxValue::F64(v) => {
                out.Push(4);
                USeg::FromLen(8).Traverse(|i| out.Push(v.to_bits().to_le_bytes()[i as usize]));
            }
            FluxValue::String(s) => {
                if s.len() + out.Size() as usize + 11 > MAX_BYTES {
                    return Err("Flux byte budget exceeded".into());
                }
                out.Push(5);
                Varint(s.len() as u64, out);
                USeg::FromLen(s.len() as u32).Traverse(|i| out.Push(s.as_bytes()[i as usize]));
            }
            FluxValue::Array(values) => {
                out.Push(6);
                let mut index = 0;
                while index < values.Size() {
                    Encode(&values[index], out, depth + 1, count)?;
                    index += 1;
                }
                out.Push(8);
            }
            FluxValue::Object(fields) => {
                out.Push(7);
                let mut keys = BTreeSet::new();
                let mut index = 0;
                while index < fields.Size() {
                    let (key, child) = &fields[index];
                    if !keys.insert(key.as_str()) {
                        return Err(format!("Duplicate Flux key: {key}"));
                    }
                    Encode(&FluxValue::String(key.clone()), out, depth + 1, count)?;
                    Encode(child, out, depth + 1, count)?;
                    index += 1;
                }
                out.Push(8);
            }
        }
        if out.Size() as usize > MAX_BYTES {
            return Err("Flux byte budget exceeded".into());
        }
        return Ok(());
    }
    let mut out = Stash::New();
    Encode(value, &mut out, 0, &mut 0)?;
    return Ok(out.IntoBuff());
}

fn TextString(value: &str, out: &mut String)
{
    out.push('"');
    value.chars().for_each(|c| {
                     match c {
                         '"' => out.push_str("\\\""),
                         '\\' => out.push_str("\\\\"),
                         '\n' => out.push_str("\\n"),
                         '\r' => out.push_str("\\r"),
                         '\t' => out.push_str("\\t"),
                         c if c < '\u{20}' => out.push_str(&format!("\\u{:04x}", c as u32)),
                         c => out.push(c),
                     }
                 });
    out.push('"');
}

fn TextValue(value: &FluxValue, out: &mut String, depth: u32) -> Result<(), String>
{
    if out.len() > MAX_BYTES {
        return Err("Flux text byte budget exceeded".into());
    }
    match value {
        FluxValue::Null => out.push_str("null"),
        FluxValue::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        FluxValue::U64(v) => out.push_str(&format!("u64({v})")),
        FluxValue::F64(v) if !v.is_finite() => {
            out.push_str(&format!("f64bits(0x{:016x})", v.to_bits()))
        }
        FluxValue::F64(v) => {
            let number = FloatLiteral(*v);
            out.push_str(&format!("f64({number})"));
        }
        FluxValue::String(v) => TextString(v, out),
        FluxValue::Array(values) => {
            out.push('[');
            let mut index = 0;
            while index < values.Size() {
                if index > 0 {
                    out.push_str(", ");
                }
                TextValue(&values[index], out, depth + 1)?;
                index += 1;
            }
            out.push(']');
        }
        FluxValue::Object(fields) => {
            out.push('{');
            let mut index = 0;
            while index < fields.Size() {
                if index > 0 {
                    out.push(',');
                }
                out.push('\n');
                USeg::FromLen((depth + 1) * 2).Traverse(|_| out.push(' '));
                TextString(&fields[index].0, out);
                out.push_str(": ");
                TextValue(&fields[index].1, out, depth + 1)?;
                index += 1;
            }
            if !fields.IsEmpty() {
                out.push('\n');
                USeg::FromLen(depth * 2).Traverse(|_| out.push(' '));
            }
            out.push('}');
        }
    }
    return Ok(());
}

//-------------------------------------------------------------------------------------------------

struct ValueParser<'a>
{
    _Bytes: &'a [u8],
    _Pos:   usize,
    _Count: u32,
}

impl<'a> ValueParser<'a>
{
    fn New(bytes: &'a [u8]) -> Self
    {
        return Self { _Bytes: bytes,
                      _Pos:   0,
                      _Count: 0, };
    }
    fn Remaining(&self) -> &'a [u8] { return &self._Bytes[self._Pos..]; }
    fn Error(&self, text: &str) -> String { return format!("{text} at byte {}", self._Pos); }
    fn Take(&mut self, count: usize) -> Result<&'a [u8], String>
    {
        let end = self._Pos
                      .checked_add(count)
                      .filter(|n| *n <= self._Bytes.len())
                      .ok_or_else(|| self.Error("Unexpected EOF"))?;
        let value = &self._Bytes[self._Pos..end];
        self._Pos = end;
        return Ok(value);
    }
    fn Varint(&mut self) -> Result<u64, String>
    {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let b = self.Take(1)?[0];
            if shift == 63 && b > 1 {
                return Err(self.Error("Varint overflow"));
            }
            value |= u64::from(b & 127) << shift;
            if b & 128 == 0 {
                if shift > 0 && b == 0 {
                    return Err(self.Error("Overlong varint"));
                }
                return Ok(value);
            }
            shift += 7;
            if shift > 63 {
                return Err(self.Error("Varint overflow"));
            }
        }
    }
    fn Binary(bytes: &'a [u8]) -> Result<FluxValue, String>
    {
        let mut parser = Self::New(bytes);
        let value = parser.BinaryValue(0)?;
        if parser._Pos != bytes.len() {
            return Err(parser.Error("Trailing Flux value bytes"));
        }
        return Ok(value);
    }
    fn BinaryValue(&mut self, depth: u32) -> Result<FluxValue, String>
    {
        FluxValue::Budget(depth, &mut self._Count)?;
        return Ok(match self.Take(1)?[0] {
            0 => FluxValue::Null,
            1 => FluxValue::Bool(false),
            2 => FluxValue::Bool(true),
            3 => FluxValue::U64(self.Varint()?),
            4 => {
                FluxValue::F64(f64::from_bits(u64::from_le_bytes(self.Take(8)?
                                                                     .try_into()
                                                                     .unwrap())))
            }
            5 => {
                let length = self.Varint()?;
                if length > MAX_BYTES as u64 {
                    return Err(self.Error("String too long"));
                }
                FluxValue::String(std::str::from_utf8(self.Take(length as usize)?).map_err(|_| "Invalid UTF-8 string")?.into())
            }
            tag @ (6 | 7) => {
                let mut values = Stash::New();
                let mut fields = Stash::New();
                let mut keys = BTreeSet::new();
                while self.Remaining().first() != Some(&8) {
                    if tag == 6 {
                        values.Push(self.BinaryValue(depth + 1)?);
                    } else {
                        let key = self.BinaryValue(depth + 1)?.Text()?.to_string();
                        if !keys.insert(key.clone()) {
                            return Err(self.Error("Duplicate Flux key"));
                        }
                        fields.Push((key, self.BinaryValue(depth + 1)?));
                    }
                }
                self.Take(1)?;
                if tag == 6 {
                    FluxValue::Array(values.IntoBuff())
                } else {
                    FluxValue::Object(fields.IntoBuff())
                }
            }
            _ => return Err(self.Error("Invalid Flux tag")),
        });
    }
    fn Space(&mut self)
    {
        loop {
            while self.Remaining()
                      .first()
                      .is_some_and(u8::is_ascii_whitespace)
            {
                self._Pos += 1;
            }
            if self.Remaining().first() != Some(&b'#') {
                return;
            }
            while self.Remaining().first().is_some_and(|b| *b != b'\n') {
                self._Pos += 1;
            }
        }
    }
    fn Token(&mut self, token: &str) -> Result<(), String>
    {
        self.Space();
        if !self.Remaining().starts_with(token.as_bytes()) {
            return Err(self.Error(&format!("Expected {token}")));
        }
        self._Pos += token.len();
        return Ok(());
    }
    fn Hex(&mut self, count: usize) -> Result<u64, String>
    {
        self.Token("0x")?;
        let text = std::str::from_utf8(self.Take(count)?).map_err(|_| "Invalid hex")?;
        return u64::from_str_radix(text, 16).map_err(|_| self.Error("Invalid hex"));
    }
    fn Quoted(&mut self) -> Result<String, String>
    {
        self.Token("\"")?;
        let mut out = String::new();
        loop {
            let b = self.Take(1)?[0];
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    match self.Take(1)?[0] {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let text =
                                std::str::from_utf8(self.Take(4)?).map_err(|_| "Invalid escape")?;
                            let mut code = u32::from_str_radix(text, 16).map_err(|_| "Invalid Unicode escape")?;
                            if (0xd800..=0xdbff).contains(&code) {
                                if self.Take(2)? != b"\\u" {
                                    return Err(self.Error("Missing low surrogate"));
                                }
                                let low = u32::from_str_radix(std::str::from_utf8(self.Take(4)?).map_err(|_| "Invalid surrogate")?, 16).map_err(|_| "Invalid surrogate")?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(self.Error("Invalid low surrogate"));
                                }
                                code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00;
                            }
                            out.push(char::from_u32(code).ok_or("Invalid Unicode scalar")?);
                        }
                        _ => return Err(self.Error("Invalid escape")),
                    }
                }
                0..=31 => return Err(self.Error("Unescaped control character")),
                32..=127 => out.push(char::from(b)),
                _ => {
                    self._Pos -= 1;
                    let length = if b < 0xe0 {
                        2
                    } else if b < 0xf0 {
                        3
                    } else {
                        4
                    };
                    let s = std::str::from_utf8(self.Take(length)?).map_err(|_| "Invalid UTF-8")?;
                    let c = s.chars().next().ok_or("Unexpected EOF")?;
                    out.push(c);
                }
            }
        }
    }
    fn Value(&mut self, depth: u32) -> Result<FluxValue, String>
    {
        FluxValue::Budget(depth, &mut self._Count)?;
        self.Space();
        let first = *self.Remaining()
                         .first()
                         .ok_or_else(|| self.Error("Expected value"))?;
        if first == b'"' {
            return Ok(FluxValue::String(self.Quoted()?));
        }
        if first == b'[' || first == b'{' {
            self._Pos += 1;
            let object = first == b'{';
            let end = if object { b'}' } else { b']' };
            let mut fields = Stash::New();
            let mut values = Stash::New();
            let mut keys = BTreeSet::new();
            self.Space();
            while self.Remaining().first() != Some(&end) {
                if object {
                    let key = self.Quoted()?;
                    self._Count += 1;
                    if !keys.insert(key.clone()) {
                        return Err(self.Error("Duplicate Flux key"));
                    }
                    self.Token(":")?;
                    fields.Push((key, self.Value(depth + 1)?));
                } else {
                    values.Push(self.Value(depth + 1)?);
                }
                self.Space();
                if self.Remaining().first() == Some(&end) {
                    break;
                }
                self.Token(",")?;
                self.Space();
            }
            self.Take(1)?;
            return Ok(if object {
                FluxValue::Object(fields.IntoBuff())
            } else {
                FluxValue::Array(values.IntoBuff())
            });
        }
        if self.Remaining().starts_with(b"f64bits") {
            self.Token("f64bits")?;
            self.Token("(")?;
            let bits = self.Hex(16)?;
            self.Token(")")?;
            return Ok(FluxValue::F64(f64::from_bits(bits)));
        }
        if self.Remaining().starts_with(b"u64") || self.Remaining().starts_with(b"f64") {
            let unsigned = first == b'u';
            self.Token(if unsigned { "u64" } else { "f64" })?;
            self.Token("(")?;
            self.Space();
            let start = self._Pos;
            while self.Remaining()
                      .first()
                      .is_some_and(|b| !b.is_ascii_whitespace() && *b != b')')
            {
                self._Pos += 1;
            }
            let number =
                std::str::from_utf8(&self._Bytes[start..self._Pos]).map_err(|_| "Invalid number")?;
            self.Token(")")?;
            if unsigned {
                if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(self.Error("Invalid u64"));
                }
                return Ok(FluxValue::U64(number.parse().map_err(|_| {
                                                            self.Error("u64 overflow")
                                                        })?));
            }
            if !DecimalNumber(number) {
                return Err(self.Error("Invalid f64 literal"));
            }
            let value: f64 = number.parse().map_err(|_| self.Error("Invalid f64"))?;
            if !value.is_finite() {
                return Err(self.Error("Use f64bits for non-finite values"));
            }
            return Ok(FluxValue::F64(value));
        }
        if self.Remaining().starts_with(b"null") {
            self.Token("null")?;
            return Ok(FluxValue::Null);
        }
        if self.Remaining().starts_with(b"true") {
            self.Token("true")?;
            return Ok(FluxValue::Bool(true));
        }
        if self.Remaining().starts_with(b"false") {
            self.Token("false")?;
            return Ok(FluxValue::Bool(false));
        }
        return Err(self.Error("Expected typed Flux value"));
    }
}

fn FloatLiteral(value: f64) -> String
{
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    let exponent: i32 = exponent.parse().unwrap();
    if !(-6..=20).contains(&exponent) {
        return format!("{mantissa}e{exponent}");
    }
    let negative = mantissa.starts_with('-');
    let digits = mantissa.trim_start_matches('-').replace('.', "");
    let point = exponent + 1;
    let mut out = if negative {
        String::from("-")
    } else {
        String::new()
    };
    if point <= 0 {
        out.push_str("0.");
        USeg::FromLen((-point) as u32).Traverse(|_| out.push('0'));
        out.push_str(&digits);
    } else if point as usize >= digits.len() {
        out.push_str(&digits);
        USeg::FromLen(point as u32 - digits.len() as u32).Traverse(|_| out.push('0'));
        out.push_str(".0");
    } else {
        out.push_str(&digits[..point as usize]);
        out.push('.');
        out.push_str(&digits[point as usize..]);
    }
    return out;
}

fn DecimalNumber(text: &str) -> bool
{
    let bytes = text.as_bytes();
    let mut pos = 0;
    if bytes.first().is_some_and(|b| *b == b'+' || *b == b'-') {
        pos += 1;
    }
    let digits = |pos: &mut usize| {
        let start = *pos;
        while bytes.get(*pos).is_some_and(u8::is_ascii_digit) {
            *pos += 1;
        }
        return *pos > start;
    };
    if !digits(&mut pos) {
        return false;
    }
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        if !digits(&mut pos) {
            return false;
        }
    }
    if bytes.get(pos).is_some_and(|b| *b == b'e' || *b == b'E') {
        pos += 1;
        if bytes.get(pos).is_some_and(|b| *b == b'+' || *b == b'-') {
            pos += 1;
        }
        if !digits(&mut pos) {
            return false;
        }
    }
    return pos == bytes.len();
}

//-------------------------------------------------------------------------------------------------
