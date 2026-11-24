// file.rs ----------------------------------------------------------------------------------------
//! Bounded native FLX/TFLX storage over the existing Flux export descriptors.
//! Files are decoded into owned staging values; callers validate their schema before publishing.

use crate::flux::{FieldExp, FieldImp, FluxError, IFluxExportSource, IFluxImportSource};
use crate::silo::{Arr, Buff, IArr, Stash, USeg};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const MAGIC: &[u8; 8] = b"TRFLX\r\n\x1a";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

//-------------------------------------------------------------------------------------------------

/// Configurable resource limits for decoding and encoding Flux files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxReadLimits
{
    pub _MaxFileBytes:       u64,
    pub _MaxHeaderBytes:     u32,
    pub _MaxMetadataBytes:   u32,
    pub _MaxStringBytes:     u32,
    pub _MaxDepth:           u32,
    pub _MaxContainerItems:  u32,
    pub _MaxTotalValues:     u32,
    pub _MaxAllocationBytes: usize,
}

impl Default for FluxReadLimits
{
    fn default() -> Self
    {
        return Self {
            _MaxFileBytes:       64 * 1024 * 1024,
            _MaxHeaderBytes:     64 * 1024,
            _MaxMetadataBytes:   1024 * 1024,
            _MaxStringBytes:     16 * 1024 * 1024,
            _MaxDepth:           64,
            _MaxContainerItems:  1_000_000,
            _MaxTotalValues:     10_000_000,
            _MaxAllocationBytes: 512 * 1024 * 1024,
        };
    }
}

impl FluxReadLimits
{
    pub const fn New() -> Self
    {
        return Self {
            _MaxFileBytes:       64 * 1024 * 1024,
            _MaxHeaderBytes:     64 * 1024,
            _MaxMetadataBytes:   1024 * 1024,
            _MaxStringBytes:     16 * 1024 * 1024,
            _MaxDepth:           64,
            _MaxContainerItems:  1_000_000,
            _MaxTotalValues:     10_000_000,
            _MaxAllocationBytes: 512 * 1024 * 1024,
        };
    }

    pub const fn Strict() -> Self
    {
        return Self {
            _MaxFileBytes:       16 * 1024 * 1024,
            _MaxHeaderBytes:     64,
            _MaxMetadataBytes:   512 * 1024,
            _MaxStringBytes:     1024 * 1024,
            _MaxDepth:           64,
            _MaxContainerItems:  100_000,
            _MaxTotalValues:     1_000_000,
            _MaxAllocationBytes: 64 * 1024 * 1024,
        };
    }

    pub const fn WithMaxFileBytes(mut self, bytes: u64) -> Self
    {
        self._MaxFileBytes = bytes;
        return self;
    }

    pub const fn WithMaxDepth(mut self, depth: u32) -> Self
    {
        self._MaxDepth = depth;
        return self;
    }

    pub const fn WithMaxTotalValues(mut self, count: u32) -> Self
    {
        self._MaxTotalValues = count;
        return self;
    }

    pub const fn WithMaxMetadataBytes(mut self, bytes: u32) -> Self
    {
        self._MaxMetadataBytes = bytes;
        return self;
    }
}

//-------------------------------------------------------------------------------------------------

/// Structured error conditions for Flux file encoding, decoding, and I/O.
#[derive(Clone, Debug, PartialEq)]
pub enum FluxFileError
{
    Io(String),
    Syntax
    {
        offset:  usize,
        message: String,
    },
    LimitExceeded
    {
        limit:  &'static str,
        actual: u64,
        max:    u64,
        offset: usize,
    },
    ChecksumMismatch
    {
        region:   &'static str,
        expected: u32,
        actual:   u32,
    },
    UnsupportedVersion
    {
        major: u16,
        minor: u16,
    },
    UnsupportedFeatures(u32),
    InvalidMetadata(String),
    DuplicateKey
    {
        key:    String,
        offset: usize,
    },
    TypeMismatch
    {
        expected: &'static str,
        found:    &'static str,
        offset:   usize,
    },
    FormatMismatch(String),
    InvalidHeader(String),
    UnexpectedEof
    {
        offset: usize,
    },
    TrailingContent
    {
        offset: usize,
    },
}

impl fmt::Display for FluxFileError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::Io(msg) => write!(f, "Flux I/O error: {msg}"),
            Self::Syntax { offset, message } => {
                write!(f, "Flux syntax error at byte {offset}: {message}")
            }
            Self::LimitExceeded { limit, actual, max, offset } => {
                write!(f,
                       "Flux limit exceeded for {limit} at byte {offset}: {actual} > {max}")
            }
            Self::ChecksumMismatch { region, expected, actual } => {
                write!(f,
                       "Flux checksum mismatch for {region}: expected 0x{expected:08x}, got 0x{actual:08x}")
            }
            Self::UnsupportedVersion { major, minor } => {
                write!(f, "Unsupported Flux version: {major}.{minor}")
            }
            Self::UnsupportedFeatures(feat) => write!(f, "Unsupported Flux features: 0x{feat:08x}"),
            Self::InvalidMetadata(msg) => write!(f, "Invalid Flux metadata: {msg}"),
            Self::DuplicateKey { key, offset } => {
                write!(f, "Duplicate Flux key '{key}' at byte {offset}")
            }
            Self::TypeMismatch { expected, found, offset } => {
                write!(f,
                       "Flux type mismatch at byte {offset}: expected {expected}, found {found}")
            }
            Self::FormatMismatch(msg) => write!(f, "Flux format mismatch: {msg}"),
            Self::InvalidHeader(msg) => write!(f, "Invalid Flux header: {msg}"),
            Self::UnexpectedEof { offset } => {
                write!(f, "Unexpected EOF in Flux file at byte {offset}")
            }
            Self::TrailingContent { offset } => {
                write!(f, "Trailing content in Flux file at byte {offset}")
            }
        }
    }
}

impl std::error::Error for FluxFileError {}

impl From<FluxFileError> for String
{
    fn from(err: FluxFileError) -> Self
    {
        return err.to_string();
    }
}

impl From<std::io::Error> for FluxFileError
{
    fn from(err: std::io::Error) -> Self
    {
        return Self::Io(err.to_string());
    }
}

impl From<FluxError> for FluxFileError
{
    fn from(err: FluxError) -> Self
    {
        return match err {
            FluxError::InvalidSyntax => {
                Self::Syntax { offset:  0,
                               message: "Invalid syntax".into(), }
            }
            FluxError::TypeMismatch => {
                Self::TypeMismatch { expected: "compatible type",
                                     found:    "incompatible type",
                                     offset:   0, }
            }
            FluxError::UnexpectedEof => Self::UnexpectedEof { offset: 0 },
            FluxError::Overflow => {
                Self::LimitExceeded { limit:  "numeric overflow",
                                      actual: 0,
                                      max:    0,
                                      offset: 0, }
            }
        };
    }
}

//-------------------------------------------------------------------------------------------------

/// Verification status of a Flux file's declared checksums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluxIntegrityStatus
{
    Verified,
    Unchecked,
    Corrupted,
}

//-------------------------------------------------------------------------------------------------

/// Standard document metadata stored in every Flux container.
#[derive(Clone, Debug, PartialEq)]
pub struct FluxFileMetadata
{
    pub _Schema:           String,
    pub _SchemaVersion:    u64,
    pub _DocumentId:       String,
    pub _DocumentRevision: u64,
    pub _CreatedUtc:       Option<String>,
    pub _ModifiedUtc:      Option<String>,
    pub _ProducerName:     String,
    pub _ProducerVersion:  String,
}

impl FluxFileMetadata
{
    pub fn Schema(&self) -> &str { return &self._Schema; }
    pub fn SchemaVersion(&self) -> u64 { return self._SchemaVersion; }
    pub fn DocumentId(&self) -> &str { return &self._DocumentId; }
    pub fn DocumentRevision(&self) -> u64 { return self._DocumentRevision; }
    pub fn CreatedUtc(&self) -> Option<&str> { return self._CreatedUtc.as_deref(); }
    pub fn ModifiedUtc(&self) -> Option<&str> { return self._ModifiedUtc.as_deref(); }
    pub fn ProducerName(&self) -> &str { return &self._ProducerName; }
    pub fn ProducerVersion(&self) -> &str { return &self._ProducerVersion; }

    pub fn Extract(metadata: &FluxValue) -> Result<Self, FluxFileError>
    {
        let schema = metadata.Get("Schema")?.Text()?;
        if schema.is_empty() {
            return Err(FluxFileError::InvalidMetadata("Schema must not be empty".into()));
        }
        let schemaVersion = metadata.Get("SchemaVersion")?.UInt()?;
        if schemaVersion == 0 {
            return Err(FluxFileError::InvalidMetadata("SchemaVersion must be greater than zero"
                                                          .into()));
        }
        let docId = metadata.Get("DocumentId")?.Text()?;
        if docId.len() != 32
           || !docId.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
           || docId.bytes().all(|b| b == b'0')
        {
            return Err(FluxFileError::InvalidMetadata("DocumentId must be 32 non-zero lowercase hex digits".into()));
        }
        let docRev = metadata.Get("DocumentRevision")?.UInt()?;
        let createdUtc = match metadata.Get("CreatedUtc") {
            Ok(FluxValue::Null) => None,
            Ok(v) => {
                let s = v.Text()?;
                FluxFile::ValidateTime(v)?;
                Some(s.to_string())
            }
            Err(_) => return Err(FluxFileError::InvalidMetadata("Missing CreatedUtc".into())),
        };
        let modifiedUtc = match metadata.Get("ModifiedUtc") {
            Ok(FluxValue::Null) => None,
            Ok(v) => {
                let s = v.Text()?;
                FluxFile::ValidateTime(v)?;
                Some(s.to_string())
            }
            Err(_) => return Err(FluxFileError::InvalidMetadata("Missing ModifiedUtc".into())),
        };
        let producer = metadata.Get("Producer")?;
        let prodName = producer.Get("Name")?.Text()?;
        let prodVer = producer.Get("Version")?.Text()?;
        if prodName.is_empty() || prodVer.is_empty() {
            return Err(FluxFileError::InvalidMetadata("Producer Name and Version must not be empty".into()));
        }
        if let Ok(extension) = metadata.Get("Extensions")
           && !matches!(extension, FluxValue::Object(_))
        {
            return Err(FluxFileError::InvalidMetadata("Flux metadata Extensions must be an object"
                                                          .into()));
        }
        return Ok(Self { _Schema:           schema.to_string(),
                         _SchemaVersion:    schemaVersion,
                         _DocumentId:       docId.to_string(),
                         _DocumentRevision: docRev,
                         _CreatedUtc:       createdUtc,
                         _ModifiedUtc:      modifiedUtc,
                         _ProducerName:     prodName.to_string(),
                         _ProducerVersion:  prodVer.to_string(), });
    }
}

//-------------------------------------------------------------------------------------------------

/// Summary record describing an inspected Flux file without full deserialization.
#[derive(Clone, Debug, PartialEq)]
pub struct FluxFileInfo
{
    pub _Format:        FluxFileFormat,
    pub _Metadata:      FluxFileMetadata,
    pub _FileBytes:     u64,
    pub _MetadataBytes: u64,
    pub _PayloadBytes:  u64,
    pub _Integrity:     FluxIntegrityStatus,
}

impl FluxFileInfo
{
    pub fn Format(&self) -> FluxFileFormat { return self._Format; }
    pub fn Metadata(&self) -> &FluxFileMetadata { return &self._Metadata; }
    pub fn FileBytes(&self) -> u64 { return self._FileBytes; }
    pub fn MetadataBytes(&self) -> u64 { return self._MetadataBytes; }
    pub fn PayloadBytes(&self) -> u64 { return self._PayloadBytes; }
    pub fn Integrity(&self) -> FluxIntegrityStatus { return self._Integrity; }
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
    pub fn FromPath(path: &Path) -> Result<Self, FluxFileError>
    {
        let extension = path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
        return match extension.as_str() {
            "flx" => Ok(Self::Binary),
            "tflx" => Ok(Self::Text),
            _ => {
                Err(FluxFileError::FormatMismatch("Choose a filename ending in .flx or .tflx"
                                                      .into()))
            }
        };
    }

    pub fn Detect(bytes: Arr<'_, u8>) -> Result<Self, FluxFileError>
    {
        let raw: &[u8] = bytes.into();
        if raw.starts_with(MAGIC) {
            return Ok(Self::Binary);
        }
        if raw.starts_with(b"TFLX 1.0\r\n") || raw.starts_with(b"TFLX 1.0\n") {
            return Ok(Self::Text);
        }
        return Err(FluxFileError::FormatMismatch("Unrecognized file format: expected TRFLX binary magic or TFLX 1.0 text preamble".into()));
    }
}

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

    pub fn Get(&self, key: &str) -> Result<&Self, FluxFileError>
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
        return Err(FluxFileError::Syntax { offset:  0,
                                           message: format!("Missing Flux field: {key}"), });
    }

    pub fn Text(&self) -> Result<&str, FluxFileError>
    {
        if let Self::String(value) = self {
            return Ok(value);
        }
        return Err(FluxFileError::TypeMismatch { expected: "String",
                                                 found:    "non-String",
                                                 offset:   0, });
    }

    pub fn UInt(&self) -> Result<u64, FluxFileError>
    {
        if let Self::U64(value) = self {
            return Ok(*value);
        }
        return Err(FluxFileError::TypeMismatch { expected: "U64",
                                                 found:    "non-U64",
                                                 offset:   0, });
    }

    pub fn Real(&self) -> Result<f64, FluxFileError>
    {
        if let Self::F64(value) = self {
            return Ok(*value);
        }
        return Err(FluxFileError::TypeMismatch { expected: "F64",
                                                 found:    "non-F64",
                                                 offset:   0, });
    }

    pub fn Boolean(&self) -> Result<bool, FluxFileError>
    {
        if let Self::Bool(value) = self {
            return Ok(*value);
        }
        return Err(FluxFileError::TypeMismatch { expected: "Bool",
                                                 found:    "non-Bool",
                                                 offset:   0, });
    }

    pub fn Items(&self) -> Result<Arr<'_, Self>, FluxFileError>
    {
        if let Self::Array(values) = self {
            return Ok(values.Arr());
        }
        return Err(FluxFileError::TypeMismatch { expected: "Array",
                                                 found:    "non-Array",
                                                 offset:   0, });
    }

    pub fn FromSource(source: &dyn IFluxExportSource) -> Result<Self, FluxFileError>
    {
        return Self::FromSourceWithLimits(source, FluxReadLimits::default());
    }

    pub fn FromSourceWithLimits(source: &dyn IFluxExportSource, limits: FluxReadLimits)
                                -> Result<Self, FluxFileError>
    {
        let mut field = FieldExp::Null;
        source.FetchFieldExp(&mut field);
        return Self::Collect(field, 0, &mut 0, limits);
    }

    fn Collect(field: FieldExp<'_>, depth: u32, count: &mut u32, limits: FluxReadLimits)
               -> Result<Self, FluxFileError>
    {
        Self::Budget(depth, count, limits)?;
        return Ok(match field {
            FieldExp::Null => Self::Null,
            FieldExp::Bool(v) => Self::Bool(v),
            FieldExp::U64(v) => Self::U64(v),
            FieldExp::F64(v) => Self::F64(v),
            FieldExp::Str(v) => {
                if v.len() as u64 > limits._MaxStringBytes as u64 {
                    return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                              actual: v.len() as u64,
                                                              max:    limits._MaxStringBytes as u64,
                                                              offset: 0, });
                }
                Self::String(v.to_string())
            }
            FieldExp::String(v) => {
                if v.len() as u64 > limits._MaxStringBytes as u64 {
                    return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                              actual: v.len() as u64,
                                                              max:    limits._MaxStringBytes as u64,
                                                              offset: 0, });
                }
                Self::String(v)
            }
            FieldExp::FluxSource(source) => {
                let mut next = FieldExp::Null;
                source.FetchFieldExp(&mut next);
                Self::Collect(next, depth + 1, count, limits)?
            }
            FieldExp::Arr(mut next) => {
                let mut values = Stash::New();
                let mut field = FieldExp::Null;
                let mut items = 0u32;
                while next(&mut field) {
                    items += 1;
                    if items > limits._MaxContainerItems {
                        return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                                  actual: items as u64,
                                                                  max:    limits._MaxContainerItems
                                                                          as u64,
                                                                  offset: 0, });
                    }
                    values.Push(Self::Collect(std::mem::replace(&mut field, FieldExp::Null),
                                              depth + 1,
                                              count,
                                              limits)?);
                }
                Self::Array(values.IntoBuff())
            }
            FieldExp::Obj(mut next) => {
                let mut fields = Stash::New();
                let mut keys = BTreeSet::new();
                let mut key = String::new();
                let mut field = FieldExp::Null;
                let mut items = 0u32;
                while next(&mut key, &mut field) {
                    items += 1;
                    if items > limits._MaxContainerItems {
                        return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                                  actual: items as u64,
                                                                  max:    limits._MaxContainerItems
                                                                          as u64,
                                                                  offset: 0, });
                    }
                    if !keys.insert(key.clone()) {
                        return Err(FluxFileError::DuplicateKey { key:    key.clone(),
                                                                 offset: 0, });
                    }
                    let value = Self::Collect(std::mem::replace(&mut field, FieldExp::Null),
                                              depth + 1,
                                              count,
                                              limits)?;
                    fields.Push((std::mem::take(&mut key), value));
                }
                Self::Object(fields.IntoBuff())
            }
        });
    }

    fn Budget(depth: u32, count: &mut u32, limits: FluxReadLimits) -> Result<(), FluxFileError>
    {
        *count += 1;
        if depth > limits._MaxDepth {
            return Err(FluxFileError::LimitExceeded { limit:  "nesting depth",
                                                      actual: depth as u64,
                                                      max:    limits._MaxDepth as u64,
                                                      offset: 0, });
        }
        if *count > limits._MaxTotalValues {
            return Err(FluxFileError::LimitExceeded { limit:  "total values",
                                                      actual: *count as u64,
                                                      max:    limits._MaxTotalValues as u64,
                                                      offset: 0, });
        }
        return Ok(());
    }

    pub fn ApplyImport(&self, target: &mut dyn IFluxImportSource) -> Result<(), FluxFileError>
    {
        let mut field = FieldImp::Null;
        target.FetchFieldImp(&mut field);
        return self.FeedFieldImp(field);
    }

    pub fn FeedFieldImp(&self, mut field: FieldImp<'_>) -> Result<(), FluxFileError>
    {
        field.Resolve();
        match (self, field) {
            (Self::Null, FieldImp::Null) => return Ok(()),
            (Self::Bool(v), f) => return f.TryPostBool(*v).map_err(FluxFileError::from),
            (Self::U64(v), f) => return f.TryPostU64(*v).map_err(FluxFileError::from),
            (Self::F64(v), f) => return f.TryPostF64(*v).map_err(FluxFileError::from),
            (Self::String(s), f) => {
                return f.TryPostString(s.clone()).map_err(FluxFileError::from);
            }
            (Self::Array(values), FieldImp::Arr(mut importer)) => {
                let mut index = 0;
                while index < values.Size() {
                    let child = importer.Next().map_err(FluxFileError::from)?;
                    values[index].FeedFieldImp(child)?;
                    index += 1;
                }
                return Ok(());
            }
            (Self::Object(fields), FieldImp::Obj(mut importer)) => {
                let mut index = 0;
                while index < fields.Size() {
                    let (key, val) = &fields[index];
                    let child = importer.Field(key.as_str()).map_err(FluxFileError::from)?;
                    val.FeedFieldImp(child)?;
                    index += 1;
                }
                return importer.Commit().map_err(FluxFileError::from);
            }
            _ => {
                return Err(FluxFileError::TypeMismatch { expected: "compatible type",
                                                         found:    "incompatible type",
                                                         offset:   0, });
            }
        }
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

pub struct FluxFile
{
    _Metadata:          FluxValue,
    _Payload:           FluxValue,
    _IntegrityVerified: bool,
}

impl FluxFile
{
    pub fn New(schema: &str, version: u64, payload: FluxValue) -> Result<Self, FluxFileError>
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

    pub fn FromParts(metadata: FluxValue, payload: FluxValue) -> Result<Self, FluxFileError>
    {
        let _ = FluxFileMetadata::Extract(&metadata)?;
        return Ok(Self { _Metadata:          metadata,
                         _Payload:           payload,
                         _IntegrityVerified: false, });
    }

    pub fn ValidateTime(value: &FluxValue) -> Result<(), FluxFileError>
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
            return Err(FluxFileError::InvalidMetadata("Invalid UTC Flux timestamp format"
                                                          .into()));
        }
        let number = |start, end| {
            text.get(start..end)
                .and_then(|s| s.parse::<u32>().ok())
                .ok_or_else(|| FluxFileError::InvalidMetadata("Invalid UTC digits".into()))
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
            return Err(FluxFileError::InvalidMetadata("Invalid UTC calendar value".into()));
        }
        return Ok(());
    }

    pub fn Metadata(&self) -> &FluxValue { return &self._Metadata; }
    pub fn Payload(&self) -> &FluxValue { return &self._Payload; }
    pub fn IntegrityVerified(&self) -> bool { return self._IntegrityVerified; }

    pub fn ImportPayload(&self, target: &mut dyn IFluxImportSource)
                         -> Result<(), FluxFileError>
    {
        return self._Payload.ApplyImport(target);
    }

    pub fn DecodeStream(input: Arr<'_, u8>, target: &mut dyn IFluxImportSource)
                        -> Result<FluxFileInfo, FluxFileError>
    {
        return Self::DecodeStreamWithLimits(input, target, FluxReadLimits::default());
    }

    pub fn DecodeStreamWithLimits(input: Arr<'_, u8>, target: &mut dyn IFluxImportSource,
                                  limits: FluxReadLimits)
                                  -> Result<FluxFileInfo, FluxFileError>
    {
        let bytes: &[u8] = input.into();
        let info = Self::InspectBytes(input, limits)?;
        if info.Format() == FluxFileFormat::Binary {
            let split = 64 + info.MetadataBytes() as usize;
            let payloadBytes = &bytes[split..split + info.PayloadBytes() as usize];
            let mut parser = ValueParser::New(payloadBytes, limits, split);
            let mut field = FieldImp::Null;
            target.FetchFieldImp(&mut field);
            parser.StreamImportBinary(field, 0)?;
            return Ok(info);
        }
        let file = Self::DecodeWithLimits(input, limits)?;
        file.ImportPayload(target)?;
        return Ok(info);
    }

    pub fn EncodeStreamBinary(schema: &str, version: u64, source: &dyn IFluxExportSource)
                              -> Result<Buff<u8>, FluxFileError>
    {
        return Self::EncodeStreamBinaryWithLimits(schema,
                                                  version,
                                                  source,
                                                  FluxReadLimits::default());
    }

    pub fn EncodeStreamBinaryWithLimits(schema: &str, version: u64,
                                        source: &dyn IFluxExportSource,
                                        limits: FluxReadLimits)
                                        -> Result<Buff<u8>, FluxFileError>
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
        let metadataBytes = BinaryBytes(&metadata, limits)?;
        let mut payloadBytes = Stash::New();
        StreamEncodeBinary(source, &mut payloadBytes, limits)?;
        let mc = Crc32c(metadataBytes.Arr());
        let pc = Crc32c(payloadBytes.Arr());
        let length = 64 + metadataBytes.Size() + payloadBytes.Size();
        if length as u64 > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: length as u64,
                                                      max:    limits._MaxFileBytes,
                                                      offset: length as usize, });
        }
        let mut header = [0u8; 64];
        header[..8].copy_from_slice(MAGIC);
        header[8..10].copy_from_slice(&1u16.to_le_bytes());
        header[12..16].copy_from_slice(&64u32.to_le_bytes());
        header[24..32].copy_from_slice(&u64::from(length).to_le_bytes());
        header[32..40].copy_from_slice(&u64::from(metadataBytes.Size()).to_le_bytes());
        header[40..48].copy_from_slice(&u64::from(payloadBytes.Size()).to_le_bytes());
        header[48..52].copy_from_slice(&mc.to_le_bytes());
        header[52..56].copy_from_slice(&pc.to_le_bytes());
        let hc = Crc32c(Arr::from(&header[..]));
        header[20..24].copy_from_slice(&hc.to_le_bytes());
        return Ok(Buff::FromDispenser(length, |i| {
            if i < 64 {
                return header[i as usize];
            }
            if i < 64 + metadataBytes.Size() {
                return metadataBytes[i - 64];
            }
            return payloadBytes[i - 64 - metadataBytes.Size()];
        }));
    }

    pub fn Encode(&self, format: FluxFileFormat) -> Result<Buff<u8>, FluxFileError>
    {
        return self.EncodeWithLimits(format, FluxReadLimits::default());
    }

    pub fn EncodeWithLimits(&self, format: FluxFileFormat, limits: FluxReadLimits)
                            -> Result<Buff<u8>, FluxFileError>
    {
        let metadata = BinaryBytes(&self._Metadata, limits)?;
        if metadata.Size() as u64 > limits._MaxMetadataBytes as u64 {
            return Err(FluxFileError::LimitExceeded { limit:  "metadata bytes",
                                                      actual: metadata.Size() as u64,
                                                      max:    limits._MaxMetadataBytes as u64,
                                                      offset: 64, });
        }
        let payload = BinaryBytes(&self._Payload, limits)?;
        let mc = Crc32c(metadata.Arr());
        let pc = Crc32c(payload.Arr());
        if format == FluxFileFormat::Text {
            let mut text = String::from("TFLX 1.0\nFeatures u64(0);\nMetadata ");
            TextValue(&self._Metadata, &mut text, 0, limits)?;
            if text.len() as u64 > limits._MaxMetadataBytes as u64 {
                return Err(FluxFileError::LimitExceeded { limit:  "text metadata bytes",
                                                          actual: text.len() as u64,
                                                          max:    limits._MaxMetadataBytes as u64,
                                                          offset: 0, });
            }
            text.push_str(";\nData ");
            TextValue(&self._Payload, &mut text, 0, limits)?;
            text.push_str(&format!(";\nIntegrity crc32c(0x{mc:08x}, 0x{pc:08x});\nEnd;\n"));
            if text.len() as u64 > limits._MaxFileBytes {
                return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                          actual: text.len() as u64,
                                                          max:    limits._MaxFileBytes,
                                                          offset: text.len(), });
            }
            return Ok(Buff::FromDispenser(text.len() as u32, |i| {
                text.as_bytes()[i as usize]
            }));
        }
        let length = 64 + metadata.Size() + payload.Size();
        if length as u64 > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: length as u64,
                                                      max:    limits._MaxFileBytes,
                                                      offset: length as usize, });
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

    pub fn Decode(input: Arr<'_, u8>) -> Result<Self, FluxFileError>
    {
        return Self::DecodeWithLimits(input, FluxReadLimits::default());
    }

    pub fn DecodeWithLimits(input: Arr<'_, u8>, limits: FluxReadLimits)
                            -> Result<Self, FluxFileError>
    {
        let bytes: &[u8] = input.into();
        if bytes.len() as u64 > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: bytes.len() as u64,
                                                      max:    limits._MaxFileBytes,
                                                      offset: 0, });
        }
        if bytes.starts_with(MAGIC) {
            if bytes.len() < 64 {
                return Err(FluxFileError::UnexpectedEof { offset: bytes.len() });
            }
            let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let wide = |offset| u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let major = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
            let minor = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
            let headerLen = word(12);
            let features = word(16);
            if major != 1 || minor != 0 {
                return Err(FluxFileError::UnsupportedVersion { major,
                                                               minor, });
            }
            if features != 0 {
                return Err(FluxFileError::UnsupportedFeatures(features));
            }
            if headerLen != 64 || bytes[56..64] != [0; 8] {
                return Err(FluxFileError::InvalidHeader("Header length must be 64 and reserved bytes zero".into()));
            }
            let mut header = [0u8; 64];
            header.copy_from_slice(&bytes[..64]);
            header[20..24].fill(0);
            let expectedHc = word(20);
            let actualHc = Crc32c(Arr::from(&header[..]));
            if actualHc != expectedHc {
                return Err(FluxFileError::ChecksumMismatch { region:   "header",
                                                             expected: expectedHc,
                                                             actual:   actualHc, });
            }
            let ml = wide(32);
            let pl = wide(40);
            let fl = wide(24);
            if ml == 0
               || pl == 0
               || fl != bytes.len() as u64
               || ml.checked_add(pl).and_then(|n| n.checked_add(64)) != Some(bytes.len() as u64)
            {
                return Err(FluxFileError::InvalidHeader("Invalid region length or file size mismatch".into()));
            }
            if ml > limits._MaxMetadataBytes as u64 {
                return Err(FluxFileError::LimitExceeded { limit:  "metadata bytes",
                                                          actual: ml,
                                                          max:    limits._MaxMetadataBytes as u64,
                                                          offset: 64, });
            }
            let split = 64 + ml as usize;
            let expectedMc = word(48);
            let actualMc = Crc32c(Arr::from(&bytes[64..split]));
            if actualMc != expectedMc {
                return Err(FluxFileError::ChecksumMismatch { region:   "metadata",
                                                             expected: expectedMc,
                                                             actual:   actualMc, });
            }
            let expectedPc = word(52);
            let actualPc = Crc32c(Arr::from(&bytes[split..]));
            if actualPc != expectedPc {
                return Err(FluxFileError::ChecksumMismatch { region:   "payload",
                                                             expected: expectedPc,
                                                             actual:   actualPc, });
            }
            let metadata = ValueParser::BinaryWithLimits(&bytes[64..split], limits, 64)?;
            let payload = ValueParser::BinaryWithLimits(&bytes[split..], limits, split)?;
            let mut file = Self::FromParts(metadata, payload)?;
            file._IntegrityVerified = true;
            return Ok(file);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| FluxFileError::Syntax {
            offset:  0,
            message: "Flux text must be UTF-8".into(),
        })?;
        let body = text.strip_prefix("TFLX 1.0\r\n")
                       .or_else(|| text.strip_prefix("TFLX 1.0\n"))
                       .ok_or_else(|| {
                           FluxFileError::FormatMismatch("Unrecognized Flux text preamble".into())
                       })?;
        let mut parser = ValueParser::New(body.as_bytes(), limits, bytes.len() - body.len());
        parser.Token("Features")?;
        let features = parser.Value(0)?.UInt()?;
        if features != 0 {
            return Err(FluxFileError::UnsupportedFeatures(features as u32));
        }
        parser.Token(";")?;
        parser.Token("Metadata")?;
        let metadata = parser.Value(0)?;
        if parser._Pos > limits._MaxMetadataBytes as usize {
            return Err(FluxFileError::LimitExceeded { limit:  "metadata bytes",
                                                      actual: parser._Pos as u64,
                                                      max:    limits._MaxMetadataBytes as u64,
                                                      offset: parser.GlobalOffset(), });
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
            let actualMc = Crc32c(BinaryBytes(&metadata, limits)?.Arr());
            let actualPc = Crc32c(BinaryBytes(&payload, limits)?.Arr());
            if actualMc != mc {
                return Err(FluxFileError::ChecksumMismatch { region:   "metadata",
                                                             expected: mc,
                                                             actual:   actualMc, });
            }
            if actualPc != pc {
                return Err(FluxFileError::ChecksumMismatch { region:   "payload",
                                                             expected: pc,
                                                             actual:   actualPc, });
            }
        }
        parser.Token(";")?;
        parser.Token("End")?;
        parser.Token(";")?;
        parser.Space();
        if parser._Pos != parser._Bytes.len() {
            return Err(FluxFileError::TrailingContent { offset: parser.GlobalOffset(), });
        }
        let mut file = Self::FromParts(metadata, payload)?;
        file._IntegrityVerified = verified;
        return Ok(file);
    }

    pub fn InspectBytes(input: Arr<'_, u8>, limits: FluxReadLimits)
                        -> Result<FluxFileInfo, FluxFileError>
    {
        let bytes: &[u8] = input.into();
        if bytes.len() as u64 > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: bytes.len() as u64,
                                                      max:    limits._MaxFileBytes,
                                                      offset: 0, });
        }
        if bytes.starts_with(MAGIC) {
            if bytes.len() < 64 {
                return Err(FluxFileError::UnexpectedEof { offset: bytes.len() });
            }
            let word = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let wide = |offset| u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let major = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
            let minor = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
            let headerLen = word(12);
            let features = word(16);
            if major != 1 || minor != 0 {
                return Err(FluxFileError::UnsupportedVersion { major,
                                                               minor, });
            }
            if features != 0 {
                return Err(FluxFileError::UnsupportedFeatures(features));
            }
            if headerLen != 64 || bytes[56..64] != [0; 8] {
                return Err(FluxFileError::InvalidHeader("Header length must be 64 and reserved bytes zero".into()));
            }
            let mut header = [0u8; 64];
            header.copy_from_slice(&bytes[..64]);
            header[20..24].fill(0);
            let expectedHc = word(20);
            let actualHc = Crc32c(Arr::from(&header[..]));
            if actualHc != expectedHc {
                return Err(FluxFileError::ChecksumMismatch { region:   "header",
                                                             expected: expectedHc,
                                                             actual:   actualHc, });
            }
            let ml = wide(32);
            let pl = wide(40);
            let fl = wide(24);
            if ml == 0
               || pl == 0
               || fl != bytes.len() as u64
               || ml.checked_add(pl).and_then(|n| n.checked_add(64)) != Some(bytes.len() as u64)
            {
                return Err(FluxFileError::InvalidHeader("Invalid region length or file size mismatch".into()));
            }
            if ml > limits._MaxMetadataBytes as u64 {
                return Err(FluxFileError::LimitExceeded { limit:  "metadata bytes",
                                                          actual: ml,
                                                          max:    limits._MaxMetadataBytes as u64,
                                                          offset: 64, });
            }
            let split = 64 + ml as usize;
            let expectedMc = word(48);
            let actualMc = Crc32c(Arr::from(&bytes[64..split]));
            if actualMc != expectedMc {
                return Err(FluxFileError::ChecksumMismatch { region:   "metadata",
                                                             expected: expectedMc,
                                                             actual:   actualMc, });
            }
            let expectedPc = word(52);
            let actualPc = Crc32c(Arr::from(&bytes[split..]));
            if actualPc != expectedPc {
                return Err(FluxFileError::ChecksumMismatch { region:   "payload",
                                                             expected: expectedPc,
                                                             actual:   actualPc, });
            }
            // Parse metadata only - avoid staging/allocating the payload AST
            let metadataVal = ValueParser::BinaryWithLimits(&bytes[64..split], limits, 64)?;
            let metadata = FluxFileMetadata::Extract(&metadataVal)?;
            return Ok(FluxFileInfo { _Format:        FluxFileFormat::Binary,
                                     _Metadata:      metadata,
                                     _FileBytes:     fl,
                                     _MetadataBytes: ml,
                                     _PayloadBytes:  pl,
                                     _Integrity:     FluxIntegrityStatus::Verified, });
        }
        let file = Self::DecodeWithLimits(input, limits)?;
        let metadata = FluxFileMetadata::Extract(file.Metadata())?;
        let integrity = if file.IntegrityVerified() {
            FluxIntegrityStatus::Verified
        } else {
            FluxIntegrityStatus::Unchecked
        };
        return Ok(FluxFileInfo { _Format:        FluxFileFormat::Text,
                                 _Metadata:      metadata,
                                 _FileBytes:     bytes.len() as u64,
                                 _MetadataBytes: 0,
                                 _PayloadBytes:  0,
                                 _Integrity:     integrity, });
    }

    pub fn Inspect(path: &Path) -> Result<FluxFileInfo, FluxFileError>
    {
        return Self::InspectWithLimits(path, FluxReadLimits::default());
    }

    pub fn InspectWithLimits(path: &Path, limits: FluxReadLimits)
                             -> Result<FluxFileInfo, FluxFileError>
    {
        let mut file = std::fs::File::open(path)?;
        let size = file.metadata()?.len();
        if size > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: size,
                                                      max:    limits._MaxFileBytes,
                                                      offset: 0, });
        }
        let mut data = Buff::FromDispenser(size as u32, |_| 0u8);
        file.read_exact(data.MutArr().into())?;
        return Self::InspectBytes(data.Arr(), limits);
    }

    pub fn Verify(path: &Path) -> Result<FluxFileInfo, FluxFileError>
    {
        return Self::VerifyWithLimits(path, FluxReadLimits::default());
    }

    pub fn VerifyWithLimits(path: &Path, limits: FluxReadLimits)
                            -> Result<FluxFileInfo, FluxFileError>
    {
        let file = Self::ReadWithLimits(path, limits)?;
        let metadata = FluxFileMetadata::Extract(file.Metadata())?;
        let integrity = if file.IntegrityVerified() {
            FluxIntegrityStatus::Verified
        } else {
            FluxIntegrityStatus::Unchecked
        };
        let size = std::fs::metadata(path)?.len();
        return Ok(FluxFileInfo { _Format:        FluxFileFormat::FromPath(path)?,
                                 _Metadata:      metadata,
                                 _FileBytes:     size,
                                 _MetadataBytes: 0,
                                 _PayloadBytes:  0,
                                 _Integrity:     integrity, });
    }

    pub fn Convert(src: &Path, dst: &Path, dst_format: FluxFileFormat)
                   -> Result<(), FluxFileError>
    {
        let file = Self::Read(src)?;
        let bytes = file.Encode(dst_format)?;
        let name = dst.file_name()
                      .ok_or_else(|| FluxFileError::Io("Choose a destination file name".into()))?
                      .to_string_lossy();
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = dst.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce));
        let mut created = false;
        let result = (|| -> Result<(), FluxFileError> {
            let mut f = std::fs::OpenOptions::new().write(true)
                                                   .create_new(true)
                                                   .open(&temp)?;
            created = true;
            f.write_all(bytes.Arr().into())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&temp, dst)?;
            return Ok(());
        })();
        if result.is_err() && created {
            let _ = std::fs::remove_file(&temp);
        }
        return result;
    }

    pub fn Read(path: &Path) -> Result<Self, FluxFileError>
    {
        return Self::ReadWithLimits(path, FluxReadLimits::default());
    }

    pub fn ReadWithLimits(path: &Path, limits: FluxReadLimits) -> Result<Self, FluxFileError>
    {
        let format = FluxFileFormat::FromPath(path)?;
        let mut file = std::fs::File::open(path)?;
        let size = file.metadata()?.len();
        if size > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: size,
                                                      max:    limits._MaxFileBytes,
                                                      offset: 0, });
        }
        let mut data = Buff::FromDispenser(size as u32, |_| 0u8);
        file.read_exact(data.MutArr().into())?;
        let bytes: &[u8] = data.Arr().into();
        if (format == FluxFileFormat::Binary) != bytes.starts_with(MAGIC) {
            return Err(FluxFileError::FormatMismatch("File contents do not match the .flx/.tflx extension".into()));
        }
        let mut trailing = [0u8];
        if file.read(&mut trailing)? != 0 {
            return Err(FluxFileError::Io("File changed while reading".into()));
        }
        return Self::DecodeWithLimits(data.Arr(), limits);
    }

    pub fn Save(&self, path: &Path) -> Result<(), FluxFileError>
    {
        let format = FluxFileFormat::FromPath(path)?;
        let bytes = self.Encode(format)?;
        let name = path.file_name()
                       .ok_or_else(|| FluxFileError::Io("Choose a file name".into()))?
                       .to_string_lossy();
        let nonce = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce));
        let mut created = false;
        let result = (|| -> Result<(), FluxFileError> {
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
        return result.map_err(|e| {
                         FluxFileError::Io(format!("Could not save {}: {e}", path.display()))
                     });
    }

    pub fn NewId() -> Result<String, FluxFileError>
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
            if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 {
                return Err(FluxFileError::Io("Could not generate a document identity".into()));
            }
        }
        #[cfg(not(windows))]
        std::fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes))
                                           .map_err(|e| FluxFileError::Io(e.to_string()))?;
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

fn BinaryBytes(value: &FluxValue, limits: FluxReadLimits) -> Result<Buff<u8>, FluxFileError>
{
    fn Encode(value: &FluxValue, out: &mut Stash<u8>, depth: u32, count: &mut u32,
              limits: FluxReadLimits)
              -> Result<(), FluxFileError>
    {
        FluxValue::Budget(depth, count, limits)?;
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
                if s.len() as u64 > limits._MaxStringBytes as u64 {
                    return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                              actual: s.len() as u64,
                                                              max:    limits._MaxStringBytes as u64,
                                                              offset: out.Size() as usize, });
                }
                out.Push(5);
                Varint(s.len() as u64, out);
                USeg::FromLen(s.len() as u32).Traverse(|i| out.Push(s.as_bytes()[i as usize]));
            }
            FluxValue::Array(values) => {
                out.Push(6);
                let mut index = 0;
                while index < values.Size() {
                    Encode(&values[index], out, depth + 1, count, limits)?;
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
                        return Err(FluxFileError::DuplicateKey { key:    key.clone(),
                                                                 offset: out.Size() as usize, });
                    }
                    Encode(&FluxValue::String(key.clone()),
                           out,
                           depth + 1,
                           count,
                           limits)?;
                    Encode(child, out, depth + 1, count, limits)?;
                    index += 1;
                }
                out.Push(8);
            }
        }
        if out.Size() as u64 > limits._MaxFileBytes {
            return Err(FluxFileError::LimitExceeded { limit:  "file bytes",
                                                      actual: out.Size() as u64,
                                                      max:    limits._MaxFileBytes,
                                                      offset: out.Size() as usize, });
        }
        return Ok(());
    }
    let mut out = Stash::New();
    Encode(value, &mut out, 0, &mut 0, limits)?;
    return Ok(out.IntoBuff());
}

//-------------------------------------------------------------------------------------------------

/// Stream-encodes an IFluxExportSource directly to binary without constructing an intermediate FluxValue tree.
pub fn StreamEncodeBinary(source: &dyn IFluxExportSource, out: &mut Stash<u8>,
                          limits: FluxReadLimits)
                          -> Result<(), FluxFileError>
{
    let mut field = FieldExp::Null;
    source.FetchFieldExp(&mut field);
    let mut count = 0u32;
    return StreamFieldExpBinary(field, out, 0, &mut count, limits);
}

fn StreamFieldExpBinary(field: FieldExp<'_>, out: &mut Stash<u8>, depth: u32, count: &mut u32,
                        limits: FluxReadLimits)
                        -> Result<(), FluxFileError>
{
    if depth > limits._MaxDepth {
        return Err(FluxFileError::LimitExceeded { limit:  "nesting depth",
                                                  actual: depth as u64,
                                                  max:    limits._MaxDepth as u64,
                                                  offset: out.Size() as usize, });
    }
    *count += 1;
    if *count > limits._MaxTotalValues {
        return Err(FluxFileError::LimitExceeded { limit:  "total values",
                                                  actual: *count as u64,
                                                  max:    limits._MaxTotalValues as u64,
                                                  offset: out.Size() as usize, });
    }
    match field {
        FieldExp::Null => out.Push(0),
        FieldExp::Bool(false) => out.Push(1),
        FieldExp::Bool(true) => out.Push(2),
        FieldExp::U64(v) => {
            out.Push(3);
            Varint(v, out);
        }
        FieldExp::F64(v) => {
            out.Push(4);
            USeg::FromLen(8).Traverse(|i| out.Push(v.to_bits().to_le_bytes()[i as usize]));
        }
        FieldExp::Str(s) => {
            if s.len() as u64 > limits._MaxStringBytes as u64 {
                return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                          actual: s.len() as u64,
                                                          max:    limits._MaxStringBytes as u64,
                                                          offset: out.Size() as usize, });
            }
            out.Push(5);
            Varint(s.len() as u64, out);
            USeg::FromLen(s.len() as u32).Traverse(|i| out.Push(s.as_bytes()[i as usize]));
        }
        FieldExp::String(s) => {
            if s.len() as u64 > limits._MaxStringBytes as u64 {
                return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                          actual: s.len() as u64,
                                                          max:    limits._MaxStringBytes as u64,
                                                          offset: out.Size() as usize, });
            }
            out.Push(5);
            Varint(s.len() as u64, out);
            USeg::FromLen(s.len() as u32).Traverse(|i| out.Push(s.as_bytes()[i as usize]));
        }
        FieldExp::FluxSource(src) => {
            let mut next = FieldExp::Null;
            src.FetchFieldExp(&mut next);
            StreamFieldExpBinary(next, out, depth + 1, count, limits)?;
        }
        FieldExp::Arr(mut next) => {
            out.Push(6);
            let mut item = FieldExp::Null;
            let mut items = 0u32;
            while next(&mut item) {
                items += 1;
                if items > limits._MaxContainerItems {
                    return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                              actual: items as u64,
                                                              max:    limits._MaxContainerItems
                                                                      as u64,
                                                              offset: out.Size() as usize, });
                }
                StreamFieldExpBinary(std::mem::replace(&mut item, FieldExp::Null),
                                     out,
                                     depth + 1,
                                     count,
                                     limits)?;
            }
            out.Push(8);
        }
        FieldExp::Obj(mut next) => {
            out.Push(7);
            let mut keys = BTreeSet::new();
            let mut key = String::new();
            let mut item = FieldExp::Null;
            let mut items = 0u32;
            while next(&mut key, &mut item) {
                items += 1;
                if items > limits._MaxContainerItems {
                    return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                              actual: items as u64,
                                                              max:    limits._MaxContainerItems
                                                                      as u64,
                                                              offset: out.Size() as usize, });
                }
                if !keys.insert(key.clone()) {
                    return Err(FluxFileError::DuplicateKey { key:    key.clone(),
                                                             offset: out.Size() as usize, });
                }
                out.Push(5);
                Varint(key.len() as u64, out);
                USeg::FromLen(key.len() as u32).Traverse(|i| out.Push(key.as_bytes()[i as usize]));
                StreamFieldExpBinary(std::mem::replace(&mut item, FieldExp::Null),
                                     out,
                                     depth + 1,
                                     count,
                                     limits)?;
                key.clear();
            }
            out.Push(8);
        }
    }
    return Ok(());
}

//-------------------------------------------------------------------------------------------------

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

fn TextValue(value: &FluxValue, out: &mut String, depth: u32, limits: FluxReadLimits)
             -> Result<(), FluxFileError>
{
    if out.len() as u64 > limits._MaxFileBytes {
        return Err(FluxFileError::LimitExceeded { limit:  "text byte budget",
                                                  actual: out.len() as u64,
                                                  max:    limits._MaxFileBytes,
                                                  offset: out.len(), });
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
                TextValue(&values[index], out, depth + 1, limits)?;
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
                TextValue(&fields[index].1, out, depth + 1, limits)?;
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
    _Bytes:      &'a [u8],
    _Pos:        usize,
    _Count:      u32,
    _Limits:     FluxReadLimits,
    _BaseOffset: usize,
}

impl<'a> ValueParser<'a>
{
    fn New(bytes: &'a [u8], limits: FluxReadLimits, baseOffset: usize) -> Self
    {
        return Self { _Bytes:      bytes,
                      _Pos:        0,
                      _Count:      0,
                      _Limits:     limits,
                      _BaseOffset: baseOffset, };
    }

    fn GlobalOffset(&self) -> usize { return self._BaseOffset + self._Pos; }
    fn Remaining(&self) -> &'a [u8] { return &self._Bytes[self._Pos..]; }

    fn Error(&self, text: &str) -> FluxFileError
    {
        return FluxFileError::Syntax { offset:  self.GlobalOffset(),
                                       message: text.to_string(), };
    }

    fn Take(&mut self, count: usize) -> Result<&'a [u8], FluxFileError>
    {
        let end = self._Pos
                      .checked_add(count)
                      .filter(|n| *n <= self._Bytes.len())
                      .ok_or_else(|| {
                          FluxFileError::UnexpectedEof { offset: self.GlobalOffset(), }
                      })?;
        let value = &self._Bytes[self._Pos..end];
        self._Pos = end;
        return Ok(value);
    }

    fn Varint(&mut self) -> Result<u64, FluxFileError>
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

    fn BinaryWithLimits(bytes: &'a [u8], limits: FluxReadLimits, baseOffset: usize)
                        -> Result<FluxValue, FluxFileError>
    {
        let mut parser = Self::New(bytes, limits, baseOffset);
        let value = parser.BinaryValue(0)?;
        if parser._Pos != bytes.len() {
            return Err(FluxFileError::TrailingContent { offset: parser.GlobalOffset(), });
        }
        return Ok(value);
    }

    fn BinaryValue(&mut self, depth: u32) -> Result<FluxValue, FluxFileError>
    {
        FluxValue::Budget(depth, &mut self._Count, self._Limits)?;
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
                if length > self._Limits._MaxStringBytes as u64 {
                    return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                              actual: length,
                                                              max:    self._Limits._MaxStringBytes
                                                                      as u64,
                                                              offset: self.GlobalOffset(), });
                }
                FluxValue::String(std::str::from_utf8(self.Take(length as usize)?).map_err(|_| {
                                                                                      self.Error("Invalid UTF-8 string")
                                                                                  })?.into())
            }
            tag @ (6 | 7) => {
                let mut values = Stash::New();
                let mut fields = Stash::New();
                let mut keys = BTreeSet::new();
                let mut items = 0u32;
                while self.Remaining().first() != Some(&8) {
                    items += 1;
                    if items > self._Limits._MaxContainerItems {
                        return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                                  actual: items as u64,
                                                                  max:    self._Limits._MaxContainerItems
                                                                          as u64,
                                                                  offset: self.GlobalOffset(), });
                    }
                    if tag == 6 {
                        values.Push(self.BinaryValue(depth + 1)?);
                    } else {
                        let key = self.BinaryValue(depth + 1)?.Text()?.to_string();
                        if !keys.insert(key.clone()) {
                            return Err(FluxFileError::DuplicateKey { key,
                                                                     offset: self.GlobalOffset(), });
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

    pub fn StreamImportBinary(&mut self, mut field: FieldImp<'_>, depth: u32)
                              -> Result<(), FluxFileError>
    {
        field.Resolve();
        if depth > self._Limits._MaxDepth {
            return Err(FluxFileError::LimitExceeded { limit:  "nesting depth",
                                                      actual: depth as u64,
                                                      max:    self._Limits._MaxDepth as u64,
                                                      offset: self.GlobalOffset(), });
        }
        self._Count += 1;
        if self._Count > self._Limits._MaxTotalValues {
            return Err(FluxFileError::LimitExceeded { limit:  "total values",
                                                      actual: self._Count as u64,
                                                      max:    self._Limits._MaxTotalValues as u64,
                                                      offset: self.GlobalOffset(), });
        }
        let tag = self.Take(1)?[0];
        match tag {
            0 => {
                match field {
                    FieldImp::Null => return Ok(()),
                    FieldImp::FluxSink(flx) => {
                        return flx.TryFromFieldImp(FieldImp::Null).map_err(FluxFileError::from);
                    }
                    FieldImp::BoxedSink(mut flx) => {
                        return flx.TryFromFieldImp(FieldImp::Null).map_err(FluxFileError::from);
                    }
                    _ => {
                        return Err(FluxFileError::TypeMismatch { expected: "Null",
                                                                 found:    "non-Null",
                                                                 offset:   self.GlobalOffset(), });
                    }
                }
            }
            1 | 2 => {
                let v = tag == 2;
                return field.TryPostBool(v).map_err(FluxFileError::from);
            }
            3 => {
                let v = self.Varint()?;
                return field.TryPostU64(v).map_err(FluxFileError::from);
            }
            4 => {
                let bits = u64::from_le_bytes(self.Take(8)?.try_into().unwrap());
                let v = f64::from_bits(bits);
                return field.TryPostF64(v).map_err(FluxFileError::from);
            }
            5 => {
                let len = self.Varint()?;
                if len > self._Limits._MaxStringBytes as u64 {
                    return Err(FluxFileError::LimitExceeded { limit:  "string length",
                                                              actual: len,
                                                              max:    self._Limits._MaxStringBytes
                                                                      as u64,
                                                              offset: self.GlobalOffset(), });
                }
                let raw = self.Take(len as usize)?;
                let s = std::str::from_utf8(raw).map_err(|_| {
                                                    FluxFileError::Syntax { offset:
                                                                                self.GlobalOffset(),
                                                                            message:
                                                                                "Invalid UTF-8 string"
                                                                                    .into(), }
                                                })?;
                return field.TryPostString(s.to_string())
                            .map_err(FluxFileError::from);
            }
            6 => {
                if let FieldImp::Arr(mut arr) = field {
                    let mut items = 0u32;
                    while self.Remaining().first() != Some(&8) {
                        items += 1;
                        if items > self._Limits._MaxContainerItems {
                            return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                                      actual: items as u64,
                                                                      max:    self._Limits
                                                                                  ._MaxContainerItems
                                                                              as u64,
                                                                      offset: self.GlobalOffset(), });
                        }
                        let child = arr.Next().map_err(FluxFileError::from)?;
                        self.StreamImportBinary(child, depth + 1)?;
                    }
                    self.Take(1)?;
                    return Ok(());
                }
                return Err(FluxFileError::TypeMismatch { expected: "Array",
                                                         found:    "non-Array",
                                                         offset:   self.GlobalOffset(), });
            }
            7 => {
                if let FieldImp::Obj(mut obj) = field {
                    let mut keys = BTreeSet::new();
                    let mut items = 0u32;
                    while self.Remaining().first() != Some(&8) {
                        items += 1;
                        if items > self._Limits._MaxContainerItems {
                            return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                                      actual: items as u64,
                                                                      max:    self._Limits
                                                                                  ._MaxContainerItems
                                                                              as u64,
                                                                      offset: self.GlobalOffset(), });
                        }
                        let keyTag = self.Take(1)?[0];
                        if keyTag != 5 {
                            return Err(FluxFileError::Syntax { offset:  self.GlobalOffset(),
                                                               message: "Object key must be a string"
                                                                            .into(), });
                        }
                        let keyLen = self.Varint()?;
                        let keyRaw = self.Take(keyLen as usize)?;
                        let key = std::str::from_utf8(keyRaw).map_err(|_| {
                                                                 FluxFileError::Syntax { offset:
                                                                                             self.GlobalOffset(),
                                                                                         message:
                                                                                             "Invalid UTF-8 object key"
                                                                                                 .into(), }
                                                             })?;
                        if !keys.insert(key.to_string()) {
                            return Err(FluxFileError::DuplicateKey { key:    key.to_string(),
                                                                     offset: self.GlobalOffset(), });
                        }
                        let child = obj.Field(key).map_err(FluxFileError::from)?;
                        self.StreamImportBinary(child, depth + 1)?;
                    }
                    self.Take(1)?;
                    return obj.Commit().map_err(FluxFileError::from);
                }
                return Err(FluxFileError::TypeMismatch { expected: "Object",
                                                         found:    "non-Object",
                                                         offset:   self.GlobalOffset(), });
            }
            _ => {
                return Err(FluxFileError::Syntax { offset:  self.GlobalOffset(),
                                                   message: format!("Invalid Flux tag: {tag}"), });
            }
        }
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

    fn Token(&mut self, token: &str) -> Result<(), FluxFileError>
    {
        self.Space();
        if !self.Remaining().starts_with(token.as_bytes()) {
            return Err(self.Error(&format!("Expected {token}")));
        }
        self._Pos += token.len();
        return Ok(());
    }

    fn Hex(&mut self, count: usize) -> Result<u64, FluxFileError>
    {
        self.Token("0x")?;
        let text =
            std::str::from_utf8(self.Take(count)?).map_err(|_| self.Error("Invalid hex"))?;
        return u64::from_str_radix(text, 16).map_err(|_| self.Error("Invalid hex"));
    }

    fn Quoted(&mut self) -> Result<String, FluxFileError>
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
                            let text = std::str::from_utf8(self.Take(4)?).map_err(|_| {
                                                                             self.Error("Invalid escape")
                                                                         })?;
                            let mut code = u32::from_str_radix(text, 16).map_err(|_| {
                                                                            self.Error("Invalid Unicode escape")
                                                                        })?;
                            if (0xd800..=0xdbff).contains(&code) {
                                if self.Take(2)? != b"\\u" {
                                    return Err(self.Error("Missing low surrogate"));
                                }
                                let low = u32::from_str_radix(std::str::from_utf8(self.Take(4)?).map_err(|_| self.Error("Invalid surrogate"))?, 16).map_err(|_| self.Error("Invalid surrogate"))?;
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return Err(self.Error("Invalid low surrogate"));
                                }
                                code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00;
                            }
                            out.push(char::from_u32(code).ok_or_else(|| {
                                                             self.Error("Invalid Unicode scalar")
                                                         })?);
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
                    let s = std::str::from_utf8(self.Take(length)?).map_err(|_| {
                                                                       self.Error("Invalid UTF-8")
                                                                   })?;
                    let c = s.chars()
                             .next()
                             .ok_or_else(|| self.Error("Unexpected EOF"))?;
                    out.push(c);
                }
            }
        }
    }

    fn Value(&mut self, depth: u32) -> Result<FluxValue, FluxFileError>
    {
        FluxValue::Budget(depth, &mut self._Count, self._Limits)?;
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
            let mut items = 0u32;
            while self.Remaining().first() != Some(&end) {
                items += 1;
                if items > self._Limits._MaxContainerItems {
                    return Err(FluxFileError::LimitExceeded { limit:  "container items",
                                                              actual: items as u64,
                                                              max:    self._Limits._MaxContainerItems
                                                                      as u64,
                                                              offset: self.GlobalOffset(), });
                }
                if object {
                    let key = self.Quoted()?;
                    self._Count += 1;
                    if !keys.insert(key.clone()) {
                        return Err(FluxFileError::DuplicateKey { key,
                                                                 offset: self.GlobalOffset(), });
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
            let number = std::str::from_utf8(&self._Bytes[start..self._Pos]).map_err(|_| {
                                                                                self.Error("Invalid number")
                                                                            })?;
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

//-------------------------------------------------------------------------------------------------

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
