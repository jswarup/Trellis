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
    fn FindSignal(&self, name: &str) -> Option<u32>;
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
