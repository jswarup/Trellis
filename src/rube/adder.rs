//-- adder.rs ------------------------------------------------------------------------------------------------------
use	crate::rube::engine::SimEngine;
use	crate::rube::gates::{ AndGate, OrGate, XorGate };
use	crate::rube::layout::Layout;
use	crate::rube::module::KernelKind;
use	crate::rube::port::{ ModuleId, PortDesc, PortId };
use	crate::silo::{ Buff, Stash, USeg };

//------------------------------------------------------------------------------------------------------------------
// 1-Bit Half Adder (XOR sum + AND carry).
// Modeled directly from Trellis `adder.h`.
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HalfAdder
{
    pub _Id:    ModuleId,
    pub _Xor:   XorGate,
    pub _And:   AndGate,
    pub _In1:   PortId,
    pub _In2:   PortId,
    pub _Sum:   PortId,
    pub _Carry: PortId,
}
impl HalfAdder
{
    #[inline]
    pub fn	New( layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent( layout, name, ModuleId::None())
    }
    pub fn	WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let  	inDescs = [PortDesc::Bool( "a"), PortDesc::Bool( "b")];
        let  	outDescs = [PortDesc::Bool( "sum"), PortDesc::Bool( "carry")];
        let  	id = layout.AddModule( 
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::None,
        );
        let  	in1 = layout.InPort( id, 0);
        let  	in2 = layout.InPort( id, 1);
        let  	sum = layout.OutPort( id, 0);
        let  	carry = layout.OutPort( id, 1);
        let  	nameStr = if name.is_empty() { "HalfAdder" } else { name };
        let  	xorName = format!( "{}.Xor", nameStr);
        let  	andName = format!( "{}.And", nameStr);
        let  	xorGate = XorGate::WithParent( layout, &xorName, id);
        let  	andGate = AndGate::WithParent( layout, &andName, id);
        layout.Connect( in1, xorGate.In1());
        layout.Connect( in2, xorGate.In2());
        layout.Connect( in1, andGate.In1());
        layout.Connect( in2, andGate.In2());
        layout.Connect( xorGate.Out(), sum);
        layout.Connect( andGate.Out(), carry);
        layout.SealModule( id);
        Self {
            _Id:    id,
            _Xor:   xorGate,
            _And:   andGate,
            _In1:   in1,
            _In2:   in2,
            _Sum:   sum,
            _Carry: carry,
        }
    }
    #[inline]
    pub const fn	Id( &self) -> ModuleId
    {
        self._Id
    }
    #[inline]
    pub const fn	In1( &self) -> PortId
    {
        self._In1
    }
    #[inline]
    pub const fn	In2( &self) -> PortId
    {
        self._In2
    }
    #[inline]
    pub const fn	Sum( &self) -> PortId
    {
        self._Sum
    }
    #[inline]
    pub const fn	Carry( &self) -> PortId
    {
        self._Carry
    }
    #[inline]
    pub fn	SetA( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._In1, val);
    }
    #[inline]
    pub fn	SetB( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._In2, val);
    }
    #[inline]
    pub fn	SetA4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._In1, if val { 1 } else { 0 }, isX, isI);
    }
    #[inline]
    pub fn	SetB4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._In2, if val { 1 } else { 0 }, isX, isI);
    }
}

//------------------------------------------------------------------------------------------------------------------
// 1-Bit Full Adder (2 HalfAdders + OR gate).
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FullAdder
{
    pub _Id:    ModuleId,
    pub _HA1:   HalfAdder,
    pub _HA2:   HalfAdder,
    pub _Or:    OrGate,
    pub _In1:   PortId,
    pub _In2:   PortId,
    pub _CIn:   PortId,
    pub _Sum:   PortId,
    pub _Carry: PortId,
}
impl FullAdder
{
    #[inline]
    pub fn	New( layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent( layout, name, ModuleId::None())
    }
    pub fn	WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let  	inDescs = [
            PortDesc::Bool( "a"),
            PortDesc::Bool( "b"),
            PortDesc::Bool( "cin"),
        ];
        let  	outDescs = [PortDesc::Bool( "sum"), PortDesc::Bool( "carry")];
        let  	id = layout.AddModule( 
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::None,
        );
        let  	in1 = layout.InPort( id, 0);
        let  	in2 = layout.InPort( id, 1);
        let  	cIn = layout.InPort( id, 2);
        let  	sum = layout.OutPort( id, 0);
        let  	carry = layout.OutPort( id, 1);
        let  	nameStr = if name.is_empty() { "FullAdder" } else { name };
        let  	ha1Name = format!( "{}.HA1", nameStr);
        let  	ha2Name = format!( "{}.HA2", nameStr);
        let  	orName = format!( "{}.Or", nameStr);
        let  	ha1 = HalfAdder::WithParent( layout, &ha1Name, id);
        let  	ha2 = HalfAdder::WithParent( layout, &ha2Name, id);
        let  	orGate = OrGate::WithParent( layout, &orName, id);
        // Pass-down
        layout.Connect( in1, ha1.In1());
        layout.Connect( in2, ha1.In2());
        layout.Connect( cIn, ha2.In2());
        // Sibling-to-sibling
        layout.Connect( ha1.Sum(), ha2.In1());
        layout.Connect( ha1.Carry(), orGate.In1());
        layout.Connect( ha2.Carry(), orGate.In2());
        // Pass-up
        layout.Connect( ha2.Sum(), sum);
        layout.Connect( orGate.Out(), carry);
        layout.SealModule( id);
        Self {
            _Id:    id,
            _HA1:   ha1,
            _HA2:   ha2,
            _Or:    orGate,
            _In1:   in1,
            _In2:   in2,
            _CIn:   cIn,
            _Sum:   sum,
            _Carry: carry,
        }
    }
    #[inline]
    pub const fn	Id( &self) -> ModuleId
    {
        self._Id
    }
    #[inline]
    pub const fn	In1( &self) -> PortId
    {
        self._In1
    }
    #[inline]
    pub const fn	In2( &self) -> PortId
    {
        self._In2
    }
    #[inline]
    pub const fn	CIn( &self) -> PortId
    {
        self._CIn
    }
    #[inline]
    pub const fn	Sum( &self) -> PortId
    {
        self._Sum
    }
    #[inline]
    pub const fn	Carry( &self) -> PortId
    {
        self._Carry
    }
    #[inline]
    pub fn	SetA( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._In1, val);
    }
    #[inline]
    pub fn	SetB( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._In2, val);
    }
    #[inline]
    pub fn	SetCIn( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._CIn, val);
    }
    #[inline]
    pub fn	SetA4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._In1, if val { 1 } else { 0 }, isX, isI);
    }
    #[inline]
    pub fn	SetB4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._In2, if val { 1 } else { 0 }, isX, isI);
    }
    #[inline]
    pub fn	SetCIn4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._CIn, if val { 1 } else { 0 }, isX, isI);
    }
}

//------------------------------------------------------------------------------------------------------------------
// N-Bit Ripple Carry Adder.
pub struct Adder< const N: usize>
{
    pub _Id:    ModuleId,
    pub _Bits:  Buff< FullAdder>,
    pub _A:     Buff< PortId>,
    pub _B:     Buff< PortId>,
    pub _CIn:   PortId,
    pub _Sum:   Buff< PortId>,
    pub _Carry: PortId,
}
impl< const N: usize> Adder< N>
{
    #[inline]
    pub fn	New( layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent( layout, name, ModuleId::None())
    }
    pub fn	WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let  	mut inDescs = Stash::< PortDesc>::New();
        let  	mut inDescNames = Stash::< String>::New();
        USeg::FromLen( N as u32).Traverse( |i| {
            inDescNames.Push( format!( "a{}", i));
        });
        USeg::FromLen( N as u32).Traverse( |i| {
            inDescNames.Push( format!( "b{}", i));
        });
        USeg::FromLen( inDescNames.Size()).Traverse( |i| {
            inDescs.Push( PortDesc::Bool( &inDescNames[i]));
        });
        inDescs.Push( PortDesc::Bool( "cin"));
        let  	mut outDescs = Stash::< PortDesc>::New();
        let  	mut outDescNames = Stash::< String>::New();
        USeg::FromLen( N as u32).Traverse( |i| {
            outDescNames.Push( format!( "sum{}", i));
        });
        USeg::FromLen( outDescNames.Size()).Traverse( |i| {
            outDescs.Push( PortDesc::Bool( &outDescNames[i]));
        });
        outDescs.Push( PortDesc::Bool( "carry"));
        let  	id = layout.AddModule( 
            name,
            parent,
            inDescs.AsArr(),
            outDescs.AsArr(),
            KernelKind::None,
        );
        let  	mut aPorts = Stash::< PortId>::New();
        let  	mut bPorts = Stash::< PortId>::New();
        USeg::FromLen( N as u32).Traverse( |i| {
            aPorts.Push( layout.InPort( id, i));
            bPorts.Push( layout.InPort( id, i + N as u32));
        });
        let  	cIn = layout.InPort( id, 2 * N as u32);
        let  	mut sumPorts = Stash::< PortId>::New();
        USeg::FromLen( N as u32).Traverse( |i| {
            sumPorts.Push( layout.OutPort( id, i));
        });
        let  	carry = layout.OutPort( id, N as u32);
        let  	mut fullAdders = Stash::< FullAdder>::New();
        let  	nameStr = if name.is_empty() { "Adder" } else { name };
        USeg::FromLen( N as u32).Traverse( |i| {
            let  	bitName = format!( "{}.Bit{}", nameStr, i);
            let  	bit = FullAdder::WithParent( layout, &bitName, id);
            layout.Connect( aPorts[i], bit.In1());
            layout.Connect( bPorts[i], bit.In2());
            if i == 0 {
                layout.Connect( cIn, bit.CIn());
            } else {
                layout.Connect( fullAdders[i - 1].Carry(), bit.CIn());
            }
            layout.Connect( bit.Sum(), sumPorts[i]);
            fullAdders.Push( bit);
        });
        if N > 0 {
            layout.Connect( fullAdders[( N - 1) as u32].Carry(), carry);
        }
        layout.SealModule( id);
        Self {
            _Id:    id,
            _Bits:  fullAdders.ExtractBuff(),
            _A:     aPorts.ExtractBuff(),
            _B:     bPorts.ExtractBuff(),
            _CIn:   cIn,
            _Sum:   sumPorts.ExtractBuff(),
            _Carry: carry,
        }
    }
    #[inline]
    pub const fn	Id( &self) -> ModuleId
    {
        self._Id
    }
    #[inline]
    pub const fn	Carry( &self) -> PortId
    {
        self._Carry
    }
    pub fn	SetA( &self, engine: &mut SimEngine, val: u64)
    {
        USeg::FromLen( N as u32).Traverse( |i| {
            let  	bit = ( ( val >> i) & 1) != 0;
            engine.SetBool( self._A[i], bit);
        });
    }
    pub fn	SetB( &self, engine: &mut SimEngine, val: u64)
    {
        USeg::FromLen( N as u32).Traverse( |i| {
            let  	bit = ( ( val >> i) & 1) != 0;
            engine.SetBool( self._B[i], bit);
        });
    }
    #[inline]
    pub fn	SetCarryIn( &self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool( self._CIn, val);
    }
    #[inline]
    pub fn	SetCarryIn4( &self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set( self._CIn, if val { 1 } else { 0 }, isX, isI);
    }
    pub fn	GetSum( &self, engine: &SimEngine) -> u64
    {
        let  	mut sum = 0u64;
        USeg::FromLen( N as u32).Traverse( |i| {
            if engine.GetBool( self._Sum[i]) {
                sum |= 1u64 << i;
            }
        });
        sum
    }
}
