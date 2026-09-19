//-- gates.rs ------------------------------------------------------------------------------------------------------
use	crate::rube::layout::Layout;
use	crate::rube::module::{ KernelKind, KernelOp };
use	crate::rube::port::{ ModuleId, PortDesc, PortId };

//------------------------------------------------------------------------------------------------------------------

pub fn	CreateGate2( 
    layout: &mut Layout,
    name: &str,
    parent: ModuleId,
    op: KernelOp,
) -> ( ModuleId, PortId, PortId, PortId)
{
    let  	inDescs = [PortDesc::Bool( "in1"), PortDesc::Bool( "in2")];
    let  	outDescs = [PortDesc::Bool( "out")];
    let  	modId = layout.AddModule( 
        name,
        parent,
        &inDescs[..],
        &outDescs[..],
        KernelKind::Fast( op),
    );
    let  	in1 = layout.InPort( modId, 0);
    let  	in2 = layout.InPort( modId, 1);
    let  	out = layout.OutPort( modId, 0);
    layout.SealModule( modId);
    ( modId, in1, in2, out)
}

//------------------------------------------------------------------------------------------------------------------

macro_rules! define_gate2 {
    ($GateName:ident, $OpVal:ident) => {
        #[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
        pub struct $GateName
        {
            pub _Id:  ModuleId,
            pub _In1: PortId,
            pub _In2: PortId,
            pub _Out: PortId,
        }
        impl $GateName
        {
            #[inline]
            pub fn	New( layout: &mut Layout, name: &str) -> Self
            {
                Self::WithParent( layout, name, ModuleId::None())
            }
            pub fn	WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
            {
                let  	( id, in1, in2, out) = CreateGate2( layout, name, parent, KernelOp::$OpVal);
                Self {
                    _Id:  id,
                    _In1: in1,
                    _In2: in2,
                    _Out: out,
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
            pub const fn	Out( &self) -> PortId
            {
                self._Out
            }
        }
    };
}
define_gate2!( NandGate, Nand);
define_gate2!( AndGate, And);
define_gate2!( OrGate, Or);
define_gate2!( XorGate, Xor);
define_gate2!( NorGate, Nor);
define_gate2!( XnorGate, Xnor);

//------------------------------------------------------------------------------------------------------------------
// 1-Input Inverter / NOT Gate
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NotGate
{
    pub _Id:  ModuleId,
    pub _In:  PortId,
    pub _Out: PortId,
}
impl NotGate
{
    #[inline]
    pub fn	New( layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent( layout, name, ModuleId::None())
    }
    pub fn	WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let  	inDescs = [PortDesc::Bool( "in")];
        let  	outDescs = [PortDesc::Bool( "out")];
        let  	id = layout.AddModule( 
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::Fast( KernelOp::Not),
        );
        let  	inPort = layout.InPort( id, 0);
        let  	outPort = layout.OutPort( id, 0);
        layout.SealModule( id);
        Self {
            _Id:  id,
            _In:  inPort,
            _Out: outPort,
        }
    }
    #[inline]
    pub const fn	Id( &self) -> ModuleId
    {
        self._Id
    }
    #[inline]
    pub const fn	In( &self) -> PortId
    {
        self._In
    }
    #[inline]
    pub const fn	Out( &self) -> PortId
    {
        self._Out
    }
}
