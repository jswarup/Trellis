// sdc.rs ------------------------------------------------------------------------------------------------------
use crate::rube::layout::Layout;
use crate::rube::port::ModuleId;
use crate::silo::{Buff, IArr, USeg};

//-------------------------------------------------------------------------------------------------

/// Represents a clock constraint (`create_clock`).
#[derive(Clone, Debug, PartialEq)]
pub struct ClockConstraint
{
    _Name:         String,
    _Period:       f64,
    _WaveformRise: f64,
    _WaveformFall: f64,
    _PortName:     String,
}

impl ClockConstraint
{
    pub fn New(name: impl Into<String>,
               period: f64,
               rise: f64,
               fall: f64,
               portName: impl Into<String>)
               -> Self
    {
        Self { _Name:         name.into(),
               _Period:       period,
               _WaveformRise: rise,
               _WaveformFall: fall,
               _PortName:     portName.into(), }
    }

    #[inline]
    pub fn Name(&self) -> &str { &self._Name }

    #[inline]
    pub const fn Period(&self) -> f64 { self._Period }

    #[inline]
    pub const fn WaveformRise(&self) -> f64 { self._WaveformRise }

    #[inline]
    pub const fn WaveformFall(&self) -> f64 { self._WaveformFall }

    #[inline]
    pub fn PortName(&self) -> &str { &self._PortName }
}

//-------------------------------------------------------------------------------------------------

/// Direction of an I/O delay constraint.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DelayKind
{
    Input,
    Output,
}

/// Timing check type: max (setup), min (hold), or both.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DelayType
{
    Max,
    Min,
    Both,
}

//-------------------------------------------------------------------------------------------------

/// Represents an I/O port delay constraint (`set_input_delay` or `set_output_delay`).
#[derive(Clone, Debug, PartialEq)]
pub struct PortDelayConstraint
{
    _Kind:      DelayKind,
    _ClockName: String,
    _Delay:     f64,
    _PortName:  String,
    _DelayType: DelayType,
}

impl PortDelayConstraint
{
    pub fn New(kind: DelayKind,
               clockName: impl Into<String>,
               delay: f64,
               portName: impl Into<String>,
               delayType: DelayType)
               -> Self
    {
        Self { _Kind:      kind,
               _ClockName: clockName.into(),
               _Delay:     delay,
               _PortName:  portName.into(),
               _DelayType: delayType, }
    }

    #[inline]
    pub const fn Kind(&self) -> DelayKind { self._Kind }

    #[inline]
    pub fn ClockName(&self) -> &str { &self._ClockName }

    #[inline]
    pub const fn Delay(&self) -> f64 { self._Delay }

    #[inline]
    pub fn PortName(&self) -> &str { &self._PortName }

    #[inline]
    pub const fn DelayType(&self) -> DelayType { self._DelayType }
}

//-------------------------------------------------------------------------------------------------

/// Kind of path timing exception.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PathKind
{
    FalsePath,
    MulticyclePath,
}

//-------------------------------------------------------------------------------------------------

/// Represents a timing exception path constraint (`set_false_path` or `set_multicycle_path`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathConstraint
{
    _Kind:    PathKind,
    _Cycles:  u32,
    _From:    Option<String>,
    _To:      Option<String>,
    _Through: Option<String>,
}

impl PathConstraint
{
    pub fn New(kind: PathKind,
               cycles: u32,
               from: Option<String>,
               to: Option<String>,
               through: Option<String>)
               -> Self
    {
        Self { _Kind:    kind,
               _Cycles:  cycles,
               _From:    from,
               _To:      to,
               _Through: through, }
    }

    #[inline]
    pub const fn Kind(&self) -> PathKind { self._Kind }

    #[inline]
    pub const fn Cycles(&self) -> u32 { self._Cycles }

    #[inline]
    pub fn From(&self) -> Option<&str> { self._From.as_deref() }

    #[inline]
    pub fn To(&self) -> Option<&str> { self._To.as_deref() }

    #[inline]
    pub fn Through(&self) -> Option<&str> { self._Through.as_deref() }
}

//-------------------------------------------------------------------------------------------------

/// Complete set of parsed Synopsys Design Constraints (SDC).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SdcConstraints
{
    _Clocks: Buff<ClockConstraint>,
    _Delays: Buff<PortDelayConstraint>,
    _Paths:  Buff<PathConstraint>,
}

impl SdcConstraints
{
    pub fn New(clocks: Buff<ClockConstraint>,
               delays: Buff<PortDelayConstraint>,
               paths: Buff<PathConstraint>)
               -> Self
    {
        Self { _Clocks: clocks,
               _Delays: delays,
               _Paths:  paths, }
    }

    #[inline]
    pub fn Clocks(&self) -> &Buff<ClockConstraint> { &self._Clocks }

    #[inline]
    pub fn Delays(&self) -> &Buff<PortDelayConstraint> { &self._Delays }

    #[inline]
    pub fn Paths(&self) -> &Buff<PathConstraint> { &self._Paths }

    pub fn FindClock(&self, name: &str) -> Option<&ClockConstraint>
    {
        let mut found_idx = u32::MAX;
        USeg::FromLen(self._Clocks.Size()).Traverse(|i| {
            if found_idx == u32::MAX && self._Clocks[i].Name() == name {
                found_idx = i;
            }
        });
        if found_idx != u32::MAX {
            Some(&self._Clocks[found_idx])
        } else {
            None
        }
    }

    pub fn FindInputDelay(&self, port: &str) -> Option<&PortDelayConstraint>
    {
        let mut found_idx = u32::MAX;
        USeg::FromLen(self._Delays.Size()).Traverse(|i| {
            let d = &self._Delays[i];
            if found_idx == u32::MAX && d.Kind() == DelayKind::Input && d.PortName() == port {
                found_idx = i;
            }
        });
        if found_idx != u32::MAX {
            Some(&self._Delays[found_idx])
        } else {
            None
        }
    }

    pub fn FindOutputDelay(&self, port: &str) -> Option<&PortDelayConstraint>
    {
        let mut found_idx = u32::MAX;
        USeg::FromLen(self._Delays.Size()).Traverse(|i| {
            let d = &self._Delays[i];
            if found_idx == u32::MAX && d.Kind() == DelayKind::Output && d.PortName() == port {
                found_idx = i;
            }
        });
        if found_idx != u32::MAX {
            Some(&self._Delays[found_idx])
        } else {
            None
        }
    }

    /// Emits standard SDC command syntax representing these constraints.
    pub fn ToSdc(&self) -> String
    {
        let mut out = String::new();
        out.push_str("# SDC Timing Constraints generated by Trellis EDA\n");
        self._Clocks.Arr().Traverse(|clk| {
            if !clk.PortName().is_empty() {
                out.push_str(&format!("create_clock -name {} -period {:.3} -waveform {{{:.3} {:.3}}} [get_ports {}]\n",
                                      clk.Name(), clk.Period(), clk.WaveformRise(), clk.WaveformFall(), clk.PortName()));
            } else {
                out.push_str(&format!("create_clock -name {} -period {:.3} -waveform {{{:.3} {:.3}}}\n",
                                      clk.Name(), clk.Period(), clk.WaveformRise(), clk.WaveformFall()));
            }
        });
        self._Delays.Arr().Traverse(|d| {
            let cmd = if d.Kind() == DelayKind::Input { "set_input_delay" } else { "set_output_delay" };
            let typ = match d.DelayType() {
                DelayType::Max => " -max",
                DelayType::Min => " -min",
                DelayType::Both => "",
            };
            out.push_str(&format!("{} -clock {}{} {:.3} [get_ports {}]\n",
                                  cmd, d.ClockName(), typ, d.Delay(), d.PortName()));
        });
        self._Paths.Arr().Traverse(|p| {
            match p.Kind() {
                PathKind::FalsePath => {
                    out.push_str("set_false_path");
                    if let Some(f) = p.From() { out.push_str(&format!(" -from [get_ports {}]", f)); }
                    if let Some(t) = p.To() { out.push_str(&format!(" -to [get_ports {}]", t)); }
                    if let Some(th) = p.Through() { out.push_str(&format!(" -through [get_pins {}]", th)); }
                    out.push('\n');
                }
                PathKind::MulticyclePath => {
                    out.push_str(&format!("set_multicycle_path {}", p.Cycles()));
                    if let Some(f) = p.From() { out.push_str(&format!(" -from [get_ports {}]", f)); }
                    if let Some(t) = p.To() { out.push_str(&format!(" -to [get_ports {}]", t)); }
                    out.push('\n');
                }
            }
        });
        out
    }

    /// Validates that all ports referenced by clock and delay constraints exist in the layout module.
    pub fn ValidateAgainstLayout(&self, layout: &Layout, top_mod: ModuleId) -> Result<(), String>
    {
        let m = layout.Module(top_mod);
        let in_count = m.InPorts().Size();
        let out_count = m.OutPorts().Size();

        let has_in_port = |name: &str| -> bool {
            let mut found = false;
            USeg::FromLen(in_count).Span(|i| {
                let pid = layout.InPort(top_mod, i);
                if layout.LocalPortName(pid) == name {
                    found = true;
                    return false;
                }
                true
            });
            found
        };

        let has_out_port = |name: &str| -> bool {
            let mut found = false;
            USeg::FromLen(out_count).Span(|i| {
                let pid = layout.OutPort(top_mod, i);
                if layout.LocalPortName(pid) == name {
                    found = true;
                    return false;
                }
                true
            });
            found
        };

        let mut error = None;
        self._Clocks.Arr().Traverse(|clk| {
            if error.is_none() && !clk.PortName().is_empty() && !has_in_port(clk.PortName()) {
                error = Some(format!("Clock '{}' attached to non-existent input port '{}'",
                                     clk.Name(), clk.PortName()));
            }
        });
        if let Some(e) = error {
            return Err(e);
        }

        self._Delays.Arr().Traverse(|d| {
            if error.is_none() {
                match d.Kind() {
                    DelayKind::Input => {
                        if !has_in_port(d.PortName()) {
                            error = Some(format!("Input delay constraint on non-existent port '{}'", d.PortName()));
                        }
                    }
                    DelayKind::Output => {
                        if !has_out_port(d.PortName()) {
                            error = Some(format!("Output delay constraint on non-existent port '{}'", d.PortName()));
                        }
                    }
                }
            }
        });
        if let Some(e) = error {
            return Err(e);
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
