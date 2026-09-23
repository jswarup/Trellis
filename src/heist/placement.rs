// placement.rs -------------------------------------------------------------------------------------------------

/// Placement policy governing chore execution and worker affinity.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChorePlacement
{
    /// Any executor worker may run the chore. Stealing is allowed.
    Any,
    /// Targeted to the designated worker for locality. Stealing allowed when designated worker is busy.
    Prefer(u32),
    /// Strictly bound to the designated worker. Stealing is NEVER allowed.
    Require(u32),
}

impl Default for ChorePlacement
{
    #[inline]
    fn default() -> Self { Self::Any }
}

impl ChorePlacement
{
    #[inline]
    pub const fn CanSteal(&self) -> bool { !matches!(self, Self::Require(_)) }

    #[inline]
    pub const fn TargetWorker(&self) -> Option<u32>
    {
        match self {
            Self::Any => None,
            Self::Prefer(w) | Self::Require(w) => Some(*w),
        }
    }

    #[inline]
    pub const fn CanExecuteOn(&self, worker_idx: u32) -> bool
    {
        match self {
            Self::Any | Self::Prefer(_) => true,
            Self::Require(w) => *w == worker_idx,
        }
    }

    #[inline]
    pub const fn IsRequired(&self) -> bool { matches!(self, Self::Require(_)) }

    /// Pack placement into a u16 for compact atomic storage:
    /// - 0x0000: Any
    /// - 0x4000 | (worker & 0x3FFF): Prefer(worker)
    /// - 0x8000 | (worker & 0x3FFF): Require(worker)
    #[inline]
    pub const fn ToPackedU16(&self) -> u16
    {
        match self {
            Self::Any => 0,
            Self::Prefer(w) => 0x4000 | ((*w as u16) & 0x3FFF),
            Self::Require(w) => 0x8000 | ((*w as u16) & 0x3FFF),
        }
    }

    /// Unpack placement from a u16.
    #[inline]
    pub const fn FromPackedU16(val: u16) -> Self
    {
        if val == 0 {
            Self::Any
        } else if (val & 0x8000) != 0 {
            Self::Require((val & 0x3FFF) as u32)
        } else if (val & 0x4000) != 0 {
            Self::Prefer((val & 0x3FFF) as u32)
        } else {
            Self::Any
        }
    }
}
