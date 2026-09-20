//-- trigger.rs ----------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::silo::{ Buff, USeg };

//------------------------------------------------------------------------------------------------------------------

pub type TriggerId = u32;
pub const PAST_X: u8 = 1 << 0;
pub const PAST_I: u8 = 1 << 1;
pub const PAST_MASK: u8 = 0b0000_0011;
pub const CURR_X: u8 = 1 << 2;
pub const CURR_I: u8 = 1 << 3;
pub const CURR_MASK: u8 = 0b0000_1100;
pub const FUTR_X: u8 = 1 << 4;
pub const FUTR_I: u8 = 1 << 5;
pub const FUTR_MASK: u8 = 0b0011_0000;

//------------------------------------------------------------------------------------------------------------------
/// Hot temporal state cell for triggers in Structure-of-Arrays (SoA) layout.
/// Modeled directly from Trellis `trigger.h`.
#[derive( Clone, Debug)]
pub struct TriggerWad< T = u64>
{
    pub _PastVals: Buff< T>,
    pub _CurrentVals: Buff< T>,
    pub _FutureVals: Buff< T>,
    pub _Flags: Buff< u8>,
    pub _SubscriberSpans: Buff< USeg>,
    pub _Subscribers: Buff< u32>,
}
impl< T: Copy + Default + 'static> Default for TriggerWad<T> {
    fn	default() -> Self
    {
        Self {
            _PastVals: Buff::New(),
            _CurrentVals: Buff::New(),
            _FutureVals: Buff::New(),
            _Flags: Buff::New(),
            _SubscriberSpans: Buff::New(),
            _Subscribers: Buff::New(),
        }
    }
}
impl< T: Copy + Default + PartialEq + 'static> TriggerWad<T> {
    pub fn	New( 
        pastVals: Buff< T>, currentVals: Buff< T>, futureVals: Buff< T>, flags: Buff< u8>,
        subscriberSpans: Buff< USeg>, subscribers: Buff< u32>,
    ) -> Self
    {
        Self {
            _PastVals: pastVals,
            _CurrentVals: currentVals,
            _FutureVals: futureVals,
            _Flags: flags,
            _SubscriberSpans: subscriberSpans,
            _Subscribers: subscribers,
        }
    }
    #[inline]
    pub fn	Size( &self) -> u32
    {
        self._PastVals.Size()
    }
    #[inline]
    pub fn	PastVal( &self, idx: TriggerId) -> T
    {
        self._PastVals[idx]
    }
    #[inline]
    pub fn	CurrentVal( &self, idx: TriggerId) -> T
    {
        self._CurrentVals[idx]
    }
    #[inline]
    pub fn	FutureVal( &self, idx: TriggerId) -> T
    {
        self._FutureVals[idx]
    }
    #[inline]
    pub fn	SetFutureVal( &mut self, idx: TriggerId, val: T)
    {
        self._FutureVals[idx] = val;
        self._Flags[idx] &= !FUTR_MASK;
    }
    #[inline]
    pub fn	SetImmediateVal( &mut self, idx: TriggerId, val: T)
    {
        self._CurrentVals[idx] = val;
        self._FutureVals[idx] = val;
        self._Flags[idx] &= !( CURR_MASK | FUTR_MASK);
    }
    #[inline]
    pub fn	Flags( &self, idx: TriggerId) -> u8
    {
        self._Flags[idx]
    }
    #[inline]
    pub fn	Advance( &mut self, idx: TriggerId) -> ( T, T)
    {
        let  	past = self._PastVals[idx];
        let  	current = self._CurrentVals[idx];
        self._PastVals[idx] = current;
        self._CurrentVals[idx] = self._FutureVals[idx];
        let  	f = self._Flags[idx];
        self._Flags[idx] = ( ( f >> 2) & 0b0000_1111) | ( f & 0b0011_0000);
        ( past, current)
    }
    pub fn	AdvanceAll( &mut self)
    {
        let  	sz = self.Size();
        if sz > 0 {
            self._PastVals.MutArr().CopyFrom( self._CurrentVals.Arr());
            self._CurrentVals.MutArr().CopyFrom( self._FutureVals.Arr());
            USeg::FromLen( sz).Traverse( |i| {
                let  	f = self._Flags[i];
                self._Flags[i] = ( ( f >> 2) & 0b0000_1111) | ( f & 0b0011_0000);
            });
        }
    }
    #[inline]
    pub fn	IsEdge( &self, idx: TriggerId) -> bool
    {
        let  	pastVal = self._PastVals[idx];
        let  	currVal = self._CurrentVals[idx];
        let  	f = self._Flags[idx];
        let  	pastFlags = f & PAST_MASK;
        let  	currFlags = ( f >> 2) & PAST_MASK;
        ( pastVal != currVal) || ( pastFlags != currFlags)
    }
    #[inline]
    pub fn	Past( &self, idx: TriggerId) -> T
    {
        self._PastVals[idx]
    }
    #[inline]
    pub fn	Current( &self, idx: TriggerId) -> T
    {
        self._CurrentVals[idx]
    }
    #[inline]
    pub fn	Future( &self, idx: TriggerId) -> T
    {
        self._FutureVals[idx]
    }
    #[inline]
    pub fn	IsCurrX( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & CURR_X) != 0
    }
    #[inline]
    pub fn	IsCurrI( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & CURR_I) != 0
    }
    #[inline]
    pub fn	IsCurrZ( &self, idx: TriggerId) -> bool
    {
        self.IsCurrI( idx)
    }
    #[inline]
    pub fn	IsCurrValid( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & ( CURR_X | CURR_I)) == 0
    }
    #[inline]
    pub fn	IsPastX( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & PAST_X) != 0
    }
    #[inline]
    pub fn	IsPastI( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & PAST_I) != 0
    }
    #[inline]
    pub fn	IsPastValid( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & ( PAST_X | PAST_I)) == 0
    }
    #[inline]
    pub fn	IsFutrX( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & FUTR_X) != 0
    }
    #[inline]
    pub fn	IsFutrI( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & FUTR_I) != 0
    }
    #[inline]
    pub fn	IsFutrValid( &self, idx: TriggerId) -> bool
    {
        ( self._Flags[idx] & ( FUTR_X | FUTR_I)) == 0
    }
    #[inline]
    pub fn	IsX( &self, idx: TriggerId) -> bool
    {
        self.IsCurrX( idx)
    }
    #[inline]
    pub fn	IsI( &self, idx: TriggerId) -> bool
    {
        self.IsCurrI( idx)
    }
    #[inline]
    pub fn	IsZ( &self, idx: TriggerId) -> bool
    {
        self.IsCurrI( idx)
    }
    #[inline]
    pub fn	IsValid( &self, idx: TriggerId) -> bool
    {
        self.IsCurrValid( idx)
    }
}
impl TriggerWad< u64>
{
    #[inline]
    pub fn	Init( &mut self, idx: TriggerId, val: u64, isX: bool, isI: bool)
    {
        self._PastVals[idx] = val;
        self._CurrentVals[idx] = val;
        self._FutureVals[idx] = val;
        let  	mut f = 0u8;
        if isX {
            f |= PAST_X | CURR_X | FUTR_X;
        }
        if isI {
            f |= PAST_I | CURR_I | FUTR_I;
        }
        self._Flags[idx] = f;
    }
    #[inline]
    pub fn	SetFuture( &mut self, idx: TriggerId, val: u64, isX: bool, isI: bool)
    {
        self._FutureVals[idx] = val;
        let  	mut f = self._Flags[idx] & !FUTR_MASK;
        if isX {
            f |= FUTR_X;
        }
        if isI {
            f |= FUTR_I;
        }
        self._Flags[idx] = f;
    }
    #[inline]
    pub fn	SetImmediate( &mut self, idx: TriggerId, val: u64, isX: bool, isI: bool)
    {
        self._CurrentVals[idx] = val;
        self._FutureVals[idx] = val;
        let  	mut f = self._Flags[idx] & !( CURR_MASK | FUTR_MASK);
        if isX {
            f |= CURR_X | FUTR_X;
        }
        if isI {
            f |= CURR_I | FUTR_I;
        }
        self._Flags[idx] = f;
    }
    #[inline]
    pub fn	IsPosedge( &self, idx: TriggerId) -> bool
    {
        let  	f = self._Flags[idx];
        if ( f & ( PAST_MASK | CURR_MASK)) != 0 {
            return false;
        }
        ( ( self._PastVals[idx] & 1) == 0) && ( ( self._CurrentVals[idx] & 1) != 0)
    }
    #[inline]
    pub fn	IsNegedge( &self, idx: TriggerId) -> bool
    {
        let  	f = self._Flags[idx];
        if ( f & ( PAST_MASK | CURR_MASK)) != 0 {
            return false;
        }
        ( ( self._PastVals[idx] & 1) != 0) && ( ( self._CurrentVals[idx] & 1) == 0)
    }
}
