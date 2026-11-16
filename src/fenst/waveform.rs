// waveform.rs -------------------------------------------------------------------------------------
//! Domain-neutral waveform & signal timeline traits for waveform inspection and viewing.
//! Decouples presentation (`fascia::waveform`) from concrete simulation formats (`rube::vcd`).

use crate::silo::Arr;
use std::sync::{OnceLock, RwLock};

/// A single signal timeline with discrete value change events.
pub trait WaveformSignal: Send + Sync
{
    fn Scope(&self) -> &str;
    fn Name(&self) -> &str;
    fn FullName(&self) -> &str;
    fn Bits(&self) -> u32;
    fn Id(&self) -> &str;
    fn Type(&self) -> &str;
    fn IsSingleBit(&self) -> bool { self.Bits() <= 1 }
    fn Changes(&self) -> Arr<'_, (u64, String)>;
    fn ChangesBetween(&self, start: u64, end: u64) -> Arr<'_, (u64, String)>;
    fn ValueAt(&self, time: u64) -> &str;
    fn PreviousChange(&self, time: u64) -> Option<u64>;
    fn NextChange(&self, time: u64) -> Option<u64>;
}

/// A collection of signals across a common timeline range.
pub trait WaveformModel: Send + Sync
{
    fn TimeMin(&self) -> u64;
    fn TimeMax(&self) -> u64;
    fn SignalCount(&self) -> u32;
    fn Signal(&self, index: u32) -> Option<&dyn WaveformSignal>;
    fn FindSignal(&self, name: &str) -> u32;
    fn Timescale(&self) -> &str { "" }
    fn NextTransition(&self, _time: u64) -> Option<u64> { None }
    fn PreviousTransition(&self, _time: u64) -> Option<u64> { None }
}

/// Provider for parsing raw file content into a dynamic `WaveformModel`.
pub trait WaveformProvider: Send + Sync
{
    fn CanLoad(&self, extension: &str) -> bool;
    fn Parse(&self, content: &str) -> Result<Box<dyn WaveformModel>, String>;
}

pub struct WaveformProviderRegistration(pub &'static dyn WaveformProvider);
inventory::collect!(WaveformProviderRegistration);

static WAVEFORM_PROVIDERS: OnceLock<RwLock<Vec<Box<dyn WaveformProvider>>>> = OnceLock::new();

fn Providers() -> &'static RwLock<Vec<Box<dyn WaveformProvider>>>
{
    WAVEFORM_PROVIDERS.get_or_init(|| RwLock::new(Vec::new()))
}

/// Registers a provider for parsing waveform file formats.
pub fn RegisterWaveformProvider(provider: Box<dyn WaveformProvider>)
{
    let mut lock = Providers().write().unwrap();
    lock.push(provider);
}

/// Parses waveform content using registered waveform providers.
pub fn ParseWaveform(extension: &str, content: &str) -> Result<Box<dyn WaveformModel>, String>
{
    for reg in inventory::iter::<WaveformProviderRegistration>
    {
        if reg.0.CanLoad(extension)
        {
            return reg.0.Parse(content);
        }
    }
    let lock = Providers().read().unwrap();
    for p in lock.iter()
    {
        if p.CanLoad(extension)
        {
            return p.Parse(content);
        }
    }
    Err(format!("No waveform provider registered for extension '{extension}'"))
}

//-------------------------------------------------------------------------------------------------

/// Display radix for multi-bit vector values in waveform viewers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaveformRadix
{
    #[default]
    Hex,
    Decimal,
    Binary,
    Ascii,
}

impl WaveformRadix
{
    pub fn Next(&self) -> Self
    {
        match self
        {
            Self::Hex => Self::Decimal,
            Self::Decimal => Self::Binary,
            Self::Binary => Self::Ascii,
            Self::Ascii => Self::Hex,
        }
    }

    pub fn AsStr(&self) -> &'static str
    {
        match self
        {
            Self::Hex => "HEX",
            Self::Decimal => "DEC",
            Self::Binary => "BIN",
            Self::Ascii => "ASC",
        }
    }

    pub fn Format(&self, raw: &str, bits: u32) -> String
    {
        let trimmed = raw.trim();
        let binStr = if let Some(stripped) = trimmed.strip_prefix('b').or_else(|| trimmed.strip_prefix('B'))
        {
            stripped
        }
        else
        {
            trimmed
        };

        if binStr.contains('x') || binStr.contains('X')
        {
            return "x".to_string();
        }
        if binStr.contains('z') || binStr.contains('Z')
        {
            return "z".to_string();
        }

        if let Ok(num) = u64::from_str_radix(binStr, 2)
        {
            match self
            {
                Self::Hex =>
                {
                    let hexDigits = bits.div_ceil(4).max(1) as usize;
                    format!("h{:0width$X}", num, width = hexDigits)
                }
                Self::Decimal => format!("{}", num),
                Self::Binary =>
                {
                    let binDigits = bits.max(1) as usize;
                    format!("b{:0width$b}", num, width = binDigits)
                }
                Self::Ascii =>
                {
                    if (32..=126).contains(&num)
                    {
                        format!("'{}'", num as u8 as char)
                    }
                    else
                    {
                        let hexDigits = bits.div_ceil(4).max(1) as usize;
                        format!("h{:0width$X}", num, width = hexDigits)
                    }
                }
            }
        }
        else
        {
            trimmed.to_string()
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Active measurement marker in waveform timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorMarker
{
    #[default]
    MarkerA,
    MarkerB,
}

impl CursorMarker
{
    pub fn Other(&self) -> Self
    {
        match self
        {
            Self::MarkerA => Self::MarkerB,
            Self::MarkerB => Self::MarkerA,
        }
    }
}

