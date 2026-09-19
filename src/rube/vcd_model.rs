//-- vcd_model.rs -------------------------------------------------------------------------------------------------
use	crate::rube::vcdio::{ VcdModel, VcdScope };
use	crate::silo::{ Arr, Buff, IArr, Stash };
use	std::collections::HashMap;

//-------------------------------------------------------------------------------------------------
/// A flattened, display-optimized timeline signal extracted from a VCD model.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdSignal
{
    pub _Scope: String,
    pub _Name: String,
    pub _FullName: String,
    pub _Bits: u32,
    pub _Id: String,
    pub _Type: String,
    pub _Changes: Buff< ( u64, String)>,
}

//-------------------------------------------------------------------------------------------------

impl VcdSignal
{
    #[inline]
    pub fn	IsSingleBit( &self) -> bool
    {
        self._Bits <= 1
    }
    /// Queries the signal value at an arbitrary simulation time using binary search.
    pub fn	ValueAt( &self, time: u64) -> &str
    {
        let  	arr = self._Changes.AsArr();
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
    pub _Signals: Buff< VcdSignal>,
    pub _TimeMin: u64,
    pub _TimeMax: u64,
    pub _Timescale: String,
    pub _Scopes: Buff< VcdScope>,
}
impl Default for VcdDisplayModel {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl VcdDisplayModel
{
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
        let  	mut foundIdx = None;
        let  	arr = self._Signals.AsArr();
        arr.USeg().Span( |idx| {
            if arr[idx]._FullName == name || arr[idx]._Name == name {
                foundIdx = Some( idx);
                return false;
            }
            true
        });
        foundIdx.map( |idx| &self._Signals[idx])
    }
    #[inline]
    pub fn	ValueAt( &self, name: &str, time: u64) -> Option< &str>
    {
        self.SignalByName( name).map( |sig| sig.ValueAt( time))
    }
    /// Constructs a flattened display model from a parsed VcdModel.
    pub fn	FromVcdModel( model: &VcdModel) -> Self
    {
        let  	mut signals: Stash< SignalAccum> = Stash::New();
        let  	mut idToIndices: HashMap< String, Stash< u32>> = HashMap::new();
        Self::CollectSignals( model._Scopes.AsArr(), "", &mut signals, &mut idToIndices);
        let  	mut timeMin = 0u64;
        let  	mut timeMax = 0u64;
        let  	stepCount = model._TimeSteps.Size();
        if stepCount > 0 {
            timeMin = model._TimeSteps[0]._Time;
            timeMax = model._TimeSteps[stepCount - 1]._Time;
        }
        model._TimeSteps.AsArr().Traverse( |ts| {
            if ts._Time > timeMax {
                timeMax = ts._Time;
            }
            ts._Values.AsArr().Traverse( |val| {
                if let  	Some( sigIndices) = idToIndices.get( &val._Id) {
                    sigIndices.AsArr().Traverse( |&idx| {
                        let  	changes = &mut signals[idx]._Changes;
                        let  	lastIdx = changes.Size();
                        if lastIdx > 0 && changes[lastIdx - 1].0 == ts._Time {
                            changes[lastIdx - 1].1 = val._ValStr.clone();
                        } else {
                            changes.Push( ( ts._Time, val._ValStr.clone()));
                        }
                    });
                }
            });
        });
        let  	mut finishedSignals = Stash::WithCapacity( signals.Size());
        signals.AsArr().Traverse( |sig| {
            finishedSignals.Push( VcdSignal {
                _Scope: sig._Scope.clone(),
                _Name: sig._Name.clone(),
                _FullName: sig._FullName.clone(),
                _Bits: sig._Bits,
                _Id: sig._Id.clone(),
                _Type: sig._Type.clone(),
                _Changes: sig._Changes.clone().IntoBuff(),
            });
        });
        Self {
            _Signals: finishedSignals.IntoBuff(),
            _TimeMin: timeMin,
            _TimeMax: timeMax,
            _Timescale: model._Timescale.clone(),
            _Scopes: model._Scopes.clone(),
        }
    }
    fn	CollectSignals( 
        scopes: Arr< '_, VcdScope>, parentPath: &str, signals: &mut Stash<SignalAccum>,
        idToIndices: &mut HashMap< String, Stash< u32>>,
    )
    {
        scopes.Traverse( |scope| {
            let  	scopePath = if parentPath.is_empty() {
                scope._Name.clone()
            } else {
                format!( "{}.{}", parentPath, scope._Name)
            };
            scope._Vars.AsArr().Traverse( |var| {
                let  	fullName = format!( "{}.{}", scopePath, var._Name);
                let  	sigIdx = signals.Size();
                idToIndices.entry( var._Id.clone()).or_default().Push( sigIdx);
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
            Self::CollectSignals( scope._Scopes.AsArr(), &scopePath, signals, idToIndices);
        });
    }
}
