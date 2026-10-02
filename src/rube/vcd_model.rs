//-- vcd_model.rs -------------------------------------------------------------------------------------------------
use	crate::rube::vcdio::{ VcdModel, VcdScope };
use	crate::silo::{ Arr, Buff, IArr, Stash };

//-------------------------------------------------------------------------------------------------
/// A flattened, display-optimized timeline signal extracted from a VCD model.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdSignal
{
    _Scope:     String,
    _Name:      String,
    _FullName:  String,
    _Bits:      u32,
    _Id:        String,
    _Type:      String,
    _Changes:   Buff< ( u64, String)>,
}

//-------------------------------------------------------------------------------------------------

impl VcdSignal
{
    pub fn  Scope( &self) -> &str
    {
        return &self._Scope;
    }

    pub fn  Name( &self) -> &str
    {
        return &self._Name;
    }

    pub fn  FullName( &self) -> &str
    {
        return &self._FullName;
    }

    pub fn  Bits( &self) -> u32
    {
        return self._Bits;
    }

    pub fn  Id( &self) -> &str
    {
        return &self._Id;
    }

    pub fn  Type( &self) -> &str
    {
        return &self._Type;
    }

    pub fn  Changes( &self) -> Arr< '_, ( u64, String)>
    {
        return self._Changes.Arr();
    }

    fn  FirstAfter( &self, time: u64) -> u32
    {
        return match self._Changes.Arr().USeg().BinarySearch( |index| self._Changes[index].0.cmp( &time)) {
            Ok( index) => index + 1,
            Err( index) => index,
        };
    }

    /// Borrows changes in (start, end], matching the visible waveform interval.
    pub fn  ChangesBetween( &self, start: u64, end: u64) -> Arr<'_, ( u64, String)>
    {
        let first   = self.FirstAfter( start);
        let last    = self.FirstAfter( end);
        return self._Changes.Arr().Slice( first, last.saturating_sub( first));
    }

    pub fn  NextChange( &self, time: u64) -> Option<u64>
    {
        return self._Changes.Arr().Get( self.FirstAfter( time)).map( |change| change.0);
    }

    pub fn  PreviousChange( &self, time: u64) -> Option<u64>
    {
        let index   = match self._Changes.Arr().USeg().BinarySearch( |index| self._Changes[index].0.cmp( &time)) {
            Ok( index) | Err( index) => index,
        };
        if index == 0
        {
            return None;
        }
        return Some( self._Changes[index - 1].0);
    }

    #[inline]
    pub fn	IsSingleBit( &self) -> bool
    {
        self._Bits <= 1
    }
    /// Queries the signal value at an arbitrary simulation time using binary search.
    pub fn	ValueAt( &self, time: u64) -> &str
    {
        let  	arr = self._Changes.Arr();
        if arr.IsEmpty() {
            return "x";
        }
        let  	res = arr.USeg().BinarySearch( |idx| arr[idx].0.cmp( &time));
        let  	changeIdx = match res {
            Ok( idx) => idx,
            Err( ins) => {
                if ins == 0 {
                    return "x";
                }
                ins - 1
            }
        };
        &self._Changes[changeIdx].1
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Default)]
struct SignalAccum
{
    _Scope: String,
    _Name: String,
    _FullName: String,
    _Bits: u32,
    _Id: String,
    _Type: String,
    _Changes: Stash< ( u64, String)>,
}

//-------------------------------------------------------------------------------------------------
/// Display-oriented data model containing all flattened signals and timeline boundaries.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdDisplayModel
{
    _Signals:   Buff< VcdSignal>,
    _TimeMin:   u64,
    _TimeMax:   u64,
    _Timescale: String,
    _Scopes:    Buff< VcdScope>,
}
impl Default for VcdDisplayModel {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl VcdDisplayModel
{
    pub fn  Signals( &self) -> Arr< '_, VcdSignal>
    {
        return self._Signals.Arr();
    }

    pub fn  TimeMin( &self) -> u64
    {
        return self._TimeMin;
    }

    pub fn  TimeMax( &self) -> u64
    {
        return self._TimeMax;
    }

    pub fn  Timescale( &self) -> &str
    {
        return &self._Timescale;
    }

    pub fn  Scopes( &self) -> Arr< '_, VcdScope>
    {
        return self._Scopes.Arr();
    }

    pub fn	New() -> Self
    {
        Self {
            _Signals: Buff::New(),
            _TimeMin: 0,
            _TimeMax: 0,
            _Timescale: String::new(),
            _Scopes: Buff::New(),
        }
    }
    #[inline]
    pub fn	SignalCount( &self) -> u32
    {
        self._Signals.Size()
    }
    #[inline]
    pub fn	Signal( &self, idx: u32) -> Option< &VcdSignal>
    {
        if idx < self._Signals.Size() {
            Some( &self._Signals[idx])
        } else {
            None
        }
    }
    pub fn	SignalByName( &self, name: &str) -> Option< &VcdSignal>
    {
        let mut foundIdx    = u32::MAX;
        let  	arr = self._Signals.Arr();
        arr.USeg().Span( |idx| {
            if arr[idx]._FullName == name || arr[idx]._Name == name {
                foundIdx = idx;
                return false;
            }
            true
        });
        return self.Signal( foundIdx);
    }
    #[inline]
    pub fn	ValueAt( &self, name: &str, time: u64) -> Option< &str>
    {
        self.SignalByName( name).map( |sig| sig.ValueAt( time))
    }
    /// Constructs a flattened display model from a parsed VcdModel.
    pub fn  FromVcdModel( model: &VcdModel) -> Self
    {
        let mut signals = Stash::New();
        Self::CollectSignals( model._Scopes.Arr(), "", &mut signals);
        // Keep declaration order for presentation; only the lookup indexes are sorted.
        let mut idOrder = Buff::FromDispenser( signals.Size(), |index| index);
        idOrder.MutArr().QSort( |&a, &b| {
            return ( &signals[a]._Id, a) < ( &signals[b]._Id, b);
        });
        let mut timeOrder = Buff::FromDispenser( model._TimeSteps.Size(), |index| index);
        timeOrder.MutArr().QSort( |&a, &b| {
            return ( model._TimeSteps[a]._Time, a) < ( model._TimeSteps[b]._Time, b);
        });
        let timeMin = if timeOrder.IsEmpty() { 0 } else { model._TimeSteps[timeOrder[0]]._Time };
        let timeMax = if timeOrder.IsEmpty() { 0 } else {
            model._TimeSteps[timeOrder[timeOrder.Size() - 1]]._Time
        };
        timeOrder.Arr().Traverse( |&step| {
            let ts  = &model._TimeSteps[step];
            ts._Values.Arr().Traverse( |val| {
                let Ok( mut first) = idOrder.Arr().USeg().BinarySearch( |index| {
                    return signals[idOrder[index]]._Id.cmp( &val._Id);
                }) else
                {
                    return;
                };
                while first > 0 && signals[idOrder[first - 1]]._Id == val._Id
                {
                    first -= 1;
                }
                while first < idOrder.Size() && signals[idOrder[first]]._Id == val._Id
                {
                    let changes = &mut signals[idOrder[first]]._Changes;
                    let count   = changes.Size();
                    if count > 0 && changes[count - 1].0 == ts._Time
                    {
                        changes[count - 1].1.clone_from( &val._ValStr);
                    }
                    else
                    {
                        changes.Push( ( ts._Time, val._ValStr.clone()));
                    }
                    first += 1;
                }
            });
        });
        let finished = Buff::FromDispenser( signals.Size(), |index| {
            let sig = std::mem::take( &mut signals[index]);
            return VcdSignal {
                _Scope:     sig._Scope,
                _Name:      sig._Name,
                _FullName:  sig._FullName,
                _Bits:      sig._Bits,
                _Id:        sig._Id,
                _Type:      sig._Type,
                _Changes:   sig._Changes.ExtractBuff(),
            };
        });
        return Self {
            _Signals:   finished,
            _TimeMin:   timeMin,
            _TimeMax:   timeMax,
            _Timescale: model._Timescale.clone(),
            _Scopes:    model._Scopes.clone(),
        };
    }
    fn	CollectSignals( 
        scopes: Arr< '_, VcdScope>, parentPath: &str, signals: &mut Stash<SignalAccum>,
    )
    {
        scopes.Traverse( |scope| {
            let  	scopePath = if parentPath.is_empty() {
                scope._Name.clone()
            } else {
                format!( "{}.{}", parentPath, scope._Name)
            };
            scope._Vars.Arr().Traverse( |var| {
                let  	fullName = format!( "{}.{}", scopePath, var._Name);
                signals.Push( SignalAccum {
                    _Scope: scopePath.clone(),
                    _Name: var._Name.clone(),
                    _FullName: fullName,
                    _Bits: var._Bits,
                    _Id: var._Id.clone(),
                    _Type: var._Type.clone(),
                    _Changes: Stash::New(),
                });
            });
            Self::CollectSignals( scope._Scopes.Arr(), &scopePath, signals);
        });
    }
}

//-------------------------------------------------------------------------------------------------
