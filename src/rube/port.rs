//-- port.rs ------------------------------------------------------------------------------------------------------------------

use crate::flux::{FieldExp, FieldImp, IFluxExportSource, IFluxImportSink, IFluxImportSource};

//-----------------------------------------------------------------------------------------------------------------------------

/// ModuleId — 32-bit module identifier.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ModuleId {
    _Id: u32,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for ModuleId {
    #[inline]
    fn default() -> Self {
        Self::Invalid()
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl ModuleId {
    #[inline]
    pub const fn New(id: u32) -> Self {
        Self { _Id: id }
    }
    #[inline]
    pub const fn Invalid() -> Self {
        Self { _Id: u32::MAX }
    }
    #[inline]
    pub const fn None() -> Self {
        Self::Invalid()
    }
    #[inline]
    pub const fn Id(&self) -> u32 {
        self._Id
    }
    #[inline]
    pub const fn IsValid(&self) -> bool {
        self._Id != u32::MAX
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl From<u32> for ModuleId {
    #[inline]
    fn from(id: u32) -> Self {
        Self::New(id)
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxExportSource for ModuleId {
    fn FetchFieldExp<'a>(&'a self, field: &mut FieldExp<'a>) {
        self._Id.FetchFieldExp(field);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSink for ModuleId {
    fn FromFieldImp(&mut self, field: FieldImp) -> bool {
        self._Id.FromFieldImp(field)
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSource for ModuleId {
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
        self._Id.FetchFieldImp(field);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Direction of a port.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PortDir {
    In,
    Out,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxExportSource for PortDir {
    fn FetchFieldExp<'a>(&'a self, field: &mut FieldExp<'a>) {
        let s = match self {
            Self::In => "In",
            Self::Out => "Out",
        };
        *field = FieldExp::Str(s);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSink for PortDir {
    fn FromFieldImp(&mut self, field: FieldImp) -> bool {
        if let FieldImp::Str(s) = field {
            *self = match *s {
                "In" => Self::In,
                "Out" => Self::Out,
                _ => return false,
            };
            return true;
        }
        false
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSource for PortDir {
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
        *field = FieldImp::FluxSink(self);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// PortId — 32-bit port index with direction encoded in bit 31.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PortId {
    _Id: u32,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for PortId {
    #[inline]
    fn default() -> Self {
        Self::Invalid()
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl PortId {
    pub const DIR_BIT: u32 = 31;
    pub const DIR_MASK: u32 = 1u32 << Self::DIR_BIT;
    pub const INDEX_MASK: u32 = !Self::DIR_MASK;
    #[inline]
    pub const fn New(raw: u32) -> Self {
        Self { _Id: raw }
    }
    #[inline]
    pub const fn In(index: u32) -> Self {
        Self {
            _Id: index & Self::INDEX_MASK,
        }
    }
    #[inline]
    pub const fn Out(index: u32) -> Self {
        Self {
            _Id: (index & Self::INDEX_MASK) | Self::DIR_MASK,
        }
    }
    #[inline]
    pub const fn Invalid() -> Self {
        Self { _Id: u32::MAX }
    }
    #[inline]
    pub const fn Raw(&self) -> u32 {
        self._Id
    }
    #[inline]
    pub const fn IsOut(&self) -> bool {
        (self._Id & Self::DIR_MASK) != 0
    }
    #[inline]
    pub const fn IsIn(&self) -> bool {
        !self.IsOut()
    }
    #[inline]
    pub const fn Index(&self) -> u32 {
        self._Id & Self::INDEX_MASK
    }
    #[inline]
    pub const fn Dir(&self) -> PortDir {
        if self.IsOut() {
            PortDir::Out
        } else {
            PortDir::In
        }
    }
    #[inline]
    pub const fn IsValid(&self) -> bool {
        self._Id != u32::MAX
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxExportSource for PortId {
    fn FetchFieldExp<'a>(&'a self, field: &mut FieldExp<'a>) {
        self._Id.FetchFieldExp(field);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSink for PortId {
    fn FromFieldImp(&mut self, field: FieldImp) -> bool {
        self._Id.FromFieldImp(field)
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IFluxImportSource for PortId {
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
        self._Id.FetchFieldImp(field);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// IPort — interface for types representing or owning a port identifier.
pub trait IPort: Copy {
    fn Id(&self) -> PortId;
    #[inline]
    fn Index(&self) -> u32 {
        self.Id().Index()
    }
    #[inline]
    fn Dir(&self) -> PortDir {
        self.Id().Dir()
    }
    #[inline]
    fn IsOut(&self) -> bool {
        self.Id().IsOut()
    }
    #[inline]
    fn IsIn(&self) -> bool {
        self.Id().IsIn()
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl IPort for PortId {
    #[inline]
    fn Id(&self) -> PortId {
        *self
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Kind of port data type.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum PortTypeKind {
    #[default]
    Bool,
    U8Val,
    U16Val,
    U32Val,
    U64Val,
    Custom,
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Port data type with bit-width representation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PortType {
    _Kind: PortTypeKind,
    _CustomBits: u32,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for PortType {
    #[inline]
    fn default() -> Self {
        Self::Bool()
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl PortType {
    #[inline]
    pub const fn Bool() -> Self {
        Self {
            _Kind: PortTypeKind::Bool,
            _CustomBits: 1,
        }
    }
    #[inline]
    pub const fn U8Val() -> Self {
        Self {
            _Kind: PortTypeKind::U8Val,
            _CustomBits: 8,
        }
    }
    #[inline]
    pub const fn U16Val() -> Self {
        Self {
            _Kind: PortTypeKind::U16Val,
            _CustomBits: 16,
        }
    }
    #[inline]
    pub const fn U32Val() -> Self {
        Self {
            _Kind: PortTypeKind::U32Val,
            _CustomBits: 32,
        }
    }
    #[inline]
    pub const fn U64Val() -> Self {
        Self {
            _Kind: PortTypeKind::U64Val,
            _CustomBits: 64,
        }
    }
    #[inline]
    pub const fn Custom(bits: u32) -> Self {
        Self {
            _Kind: PortTypeKind::Custom,
            _CustomBits: bits,
        }
    }
    #[inline]
    pub const fn Kind(&self) -> PortTypeKind {
        self._Kind
    }
    #[inline]
    pub const fn Bits(&self) -> u32 {
        match self._Kind {
            PortTypeKind::Bool => 1,
            PortTypeKind::U8Val => 8,
            PortTypeKind::U16Val => 16,
            PortTypeKind::U32Val => 32,
            PortTypeKind::U64Val => 64,
            PortTypeKind::Custom => self._CustomBits,
        }
    }
    #[inline]
    pub const fn TypeSize(&self) -> u32 {
        match self._Kind {
            PortTypeKind::Bool
            | PortTypeKind::U8Val
            | PortTypeKind::U16Val
            | PortTypeKind::U32Val => 1,
            PortTypeKind::U64Val => 2,
            PortTypeKind::Custom => self._CustomBits.div_ceil(32),
        }
    }
    #[inline]
    pub const fn Mask(&self) -> u64 {
        match self._Kind {
            PortTypeKind::Bool => 1u64,
            PortTypeKind::U8Val => 0xFFu64,
            PortTypeKind::U16Val => 0xFFFFu64,
            PortTypeKind::U32Val => 0xFFFF_FFFFu64,
            PortTypeKind::U64Val => u64::MAX,
            PortTypeKind::Custom => {
                if self._CustomBits >= 64 {
                    u64::MAX
                } else if self._CustomBits == 0 {
                    0u64
                } else {
                    (1u64 << self._CustomBits) - 1u64
                }
            }
        }
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Port descriptor defining name, type, and owning module.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct PortDesc {
    _Name: String,
    _Type: PortType,
    _Owner: ModuleId,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl PortDesc {
    #[inline]
    pub fn New(name: impl Into<String>, portType: PortType, owner: ModuleId) -> Self {
        Self {
            _Name: name.into(),
            _Type: portType,
            _Owner: owner,
        }
    }
    #[inline]
    pub fn WithType(name: impl Into<String>, portType: PortType) -> Self {
        Self::New(name, portType, ModuleId::Invalid())
    }
    #[inline]
    pub fn Bool(name: impl Into<String>) -> Self {
        Self::New(name, PortType::Bool(), ModuleId::Invalid())
    }
    #[inline]
    pub fn U8(name: impl Into<String>) -> Self {
        Self::New(name, PortType::U8Val(), ModuleId::Invalid())
    }
    #[inline]
    pub fn U16(name: impl Into<String>) -> Self {
        Self::New(name, PortType::U16Val(), ModuleId::Invalid())
    }
    #[inline]
    pub fn U32(name: impl Into<String>) -> Self {
        Self::New(name, PortType::U32Val(), ModuleId::Invalid())
    }
    #[inline]
    pub fn U64(name: impl Into<String>) -> Self {
        Self::New(name, PortType::U64Val(), ModuleId::Invalid())
    }
    #[inline]
    pub fn Custom(name: impl Into<String>, bits: u32) -> Self {
        Self::New(name, PortType::Custom(bits), ModuleId::Invalid())
    }
    #[inline]
    pub fn Name(&self) -> &str {
        &self._Name
    }
    #[inline]
    pub fn Type(&self) -> PortType {
        self._Type
    }
    #[inline]
    pub fn Owner(&self) -> ModuleId {
        self._Owner
    }
    #[inline]
    pub fn SetName(&mut self, name: String) {
        self._Name = name;
    }
    #[inline]
    pub fn SetOwner(&mut self, owner: ModuleId) {
        self._Owner = owner;
    }
}

//-----------------------------------------------------------------------------------------------------------------------------
