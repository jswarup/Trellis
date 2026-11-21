// src/karst/address.rs
use crate::karst::config::{ K_DIE_ADDR_BIT, K_MC_ADDR_SHIFT };

//-------------------------------------------------------------------------------------------------
// Stable wire codes in fault-response payloads. Zero is not a fault code.
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
#[repr( u32)]
pub enum MemoryFault
{
    Unaligned = 1,
    AddressWidth = 2,
    OutOfBounds = 3,
    ReadFailed = 4,
    WriteFailed = 5,
    UnknownFault = 6,
}

impl MemoryFault
{
    pub const fn FromCode( code: u32) -> Self
    {
        match code
        {
            1 => Self::Unaligned,
            2 => Self::AddressWidth,
            3 => Self::OutOfBounds,
            4 => Self::ReadFailed,
            5 => Self::WriteFailed,
            _ => Self::UnknownFault,
        }
    }

    pub const fn Message( self) -> &'static str
    {
        match self
        {
            Self::Unaligned => "Unaligned word access",
            Self::AddressWidth => "Address exceeds 24-bit flit width",
            Self::OutOfBounds => "Out of bounds memory access",
            Self::ReadFailed => "Failed to read from compute buffer",
            Self::WriteFailed => "Failed to write to compute buffer",
            Self::UnknownFault => "Unknown memory fault code",
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Remove die bit 12 and MC bits 11:10, retaining the high address bits.
/// Eight 4-KiB channels expose 32 KiB in 1-KiB stripes, without modulo aliasing.
/// Validate before packing: a flit cannot preserve bits above its 24-bit address.
pub fn DecodeLocalWord( addr: u32, channelCapacity: u64) -> Result< u32, MemoryFault>
{
    if addr > 0x00FF_FFFF
    {
        return Err( MemoryFault::AddressWidth);
    }
    let local = (( addr >> ( K_DIE_ADDR_BIT + 1)) << K_MC_ADDR_SHIFT)
        | ( addr & (( 1 << K_MC_ADDR_SHIFT) - 1));
    ValidateLocalWord( local, channelCapacity)?;
    Ok( local)
}

pub fn ValidateLocalWord( addr: u32, capacity: u64) -> Result< (), MemoryFault>
{
    if !addr.is_multiple_of( 4)
    {
        return Err( MemoryFault::Unaligned);
    }
    if u64::from( addr) + 4 > capacity
    {
        return Err( MemoryFault::OutOfBounds);
    }
    Ok( ())
}

//-------------------------------------------------------------------------------------------------
