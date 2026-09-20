//-- module.rs ------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::flux::{ FieldExp, FieldImp, IFluxExportSource, IFluxImportSink, IFluxImportSource };
use	crate::rube::coro_kernel::CoroKernelFactory;
use	crate::rube::port::ModuleId;
use	crate::rube::trigger::TriggerId;
use	crate::silo::{ Buff, USeg };
use	std::sync::Arc;

//------------------------------------------------------------------------------------------------------------------
/// Kernel operation types for fast gate/arithmetic modules.
/// Modeled directly from Trellis `module.h`.
#[derive( Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum KernelOp {
    #[default]
    Nand,
    And,
    Or,
    Not,
    Xor,
    Nor,
    Xnor,
    Add,
    Sub,
    Shl,
    Shr,
}
impl KernelOp
{
    #[inline]
    pub fn	EvalRaw( self, in1: u64, in2: u64, mask: u64) -> u64
    {
        let  	res = match self {
            Self::Nand => !( in1 & in2),
            Self::And => in1 & in2,
            Self::Or => in1 | in2,
            Self::Not => !in1,
            Self::Xor => in1 ^ in2,
            Self::Nor => !( in1 | in2),
            Self::Xnor => !( in1 ^ in2),
            Self::Add => in1.wrapping_add( in2),
            Self::Sub => in1.wrapping_sub( in2),
            Self::Shl => in1.wrapping_shl( ( in2 & 63) as u32),
            Self::Shr => in1.wrapping_shr( ( in2 & 63) as u32),
        };
        res & mask
    }
}
#[inline]
pub fn	EvalRaw( op: KernelOp, in1: u64, in2: u64, mask: u64) -> u64
{
    op.EvalRaw( in1, in2, mask)
}
impl IFluxExportSource for KernelOp {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut FieldExp< 'a>) {
        let  	s = match self {
            Self::Nand => "Nand",
            Self::And => "And",
            Self::Or => "Or",
            Self::Not => "Not",
            Self::Xor => "Xor",
            Self::Nor => "Nor",
            Self::Xnor => "Xnor",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Shl => "Shl",
            Self::Shr => "Shr",
        };
        *field = FieldExp::Str( s);
    }
}
impl IFluxImportSink for KernelOp {
    fn	FromFieldImp( &mut self, field: FieldImp) -> bool
    {
        if let  	FieldImp::Str( s) = field {
            *self = match *s {
                "Nand" => Self::Nand,
                "And" => Self::And,
                "Or" => Self::Or,
                "Not" => Self::Not,
                "Xor" => Self::Xor,
                "Nor" => Self::Nor,
                "Xnor" => Self::Xnor,
                "Add" => Self::Add,
                "Sub" => Self::Sub,
                "Shl" => Self::Shl,
                "Shr" => Self::Shr,
                _ => return false,
            };
            return true;
        }
        false
    }
}
impl IFluxImportSource for KernelOp {
    fn	FetchFieldImp< 'a>(&'a mut self, field: &mut FieldImp< 'a>) {
        *field = FieldImp::FluxSink( self);
    }
}

//------------------------------------------------------------------------------------------------------------------

#[derive( Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Eval4Result
{
    pub _Val: u64,
    pub _IsX: bool,
    pub _IsI: bool,
}
#[inline]
#[allow( clippy::too_many_arguments)] // Mirrors the compact two-input four-state kernel representation.
pub fn	Eval4State( 
    op: KernelOp, in1: u64, x1: bool, i1: bool, in2: u64, x2: bool, i2: bool, mask: u64,
) -> Eval4Result
{
    let  	mut res = Eval4Result::default();
    match op {
        KernelOp::Nand | KernelOp::And => {
            let  	isFalse1 = !x1 && !i1 && ( ( in1 & 1) == 0);
            let  	isFalse2 = !x2 && !i2 && ( ( in2 & 1) == 0);
            if isFalse1 || isFalse2 {
                res._Val = if op == KernelOp::And { 0 } else { 1 };
                res._IsX = false;
                res._IsI = false;
            } else if x1 || i1 || x2 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                let  	andVal = in1 & in2;
                res._Val = if op == KernelOp::And { andVal } else { !andVal };
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Nor | KernelOp::Or => {
            let  	isTrue1 = !x1 && !i1 && ( ( in1 & 1) != 0);
            let  	isTrue2 = !x2 && !i2 && ( ( in2 & 1) != 0);
            if isTrue1 || isTrue2 {
                res._Val = if op == KernelOp::Or { 1 } else { 0 };
                res._IsX = false;
                res._IsI = false;
            } else if x1 || i1 || x2 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                let  	orVal = in1 | in2;
                res._Val = if op == KernelOp::Or { orVal } else { !orVal };
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Not => {
            if x1 || i1 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                if in1 == 1 {
                    res._Val = 0;
                } else if in1 == 0 {
                    res._Val = 1;
                } else {
                    res._Val = !in1;
                }
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Xor => {
            if x1 || i1 || x2 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = in1 ^ in2;
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Xnor => {
            if x1 || i1 || x2 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = !( in1 ^ in2);
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Add => {
            if x1 || x2 || i1 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = in1.wrapping_add( in2);
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Sub => {
            if x1 || x2 || i1 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = in1.wrapping_sub( in2);
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Shl => {
            if x1 || x2 || i1 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = in1.wrapping_shl( ( in2 & 63) as u32);
                res._IsX = false;
                res._IsI = false;
            }
        }
        KernelOp::Shr => {
            if x1 || x2 || i1 || i2 {
                res._Val = 0;
                res._IsX = true;
                res._IsI = false;
            } else {
                res._Val = in1.wrapping_shr( ( in2 & 63) as u32);
                res._IsX = false;
                res._IsI = false;
            }
        }
    }
    res._Val &= mask;
    res
}

//------------------------------------------------------------------------------------------------------------------
/// Kernel classification for simulation execution.
#[derive( Clone)]
pub enum KernelKind {
    None,
    Fast( KernelOp),
    Coro( CoroKernelFactory),
}
impl Default for KernelKind {
    #[inline]
    fn	default() -> Self
    {
        Self::None
    }
}
impl KernelKind
{
    #[inline]
    pub const fn	IsNone( &self) -> bool
    {
        matches!( self, Self::None)
    }
    #[inline]
    pub const fn	IsCoro( &self) -> bool
    {
        matches!( self, Self::Coro( _))
    }
    #[inline]
    pub const fn	ToFastOp( &self) -> Option< KernelOp>
    {
        match self {
            Self::Fast( op) => Some( *op),
            _ => None,
        }
    }
    #[inline]
    pub fn	ClassKey( &self) -> ( u8, usize)
    {
        match self {
            Self::None => ( 2, 0),
            Self::Fast( op) => ( 0, *op as usize),
            Self::Coro( factory) => {
                let  	rawDyn: *const ( 
                    dyn Fn() -> crate::rube::coro_kernel::CoroInstance + Send + Sync
                ) = Arc::as_ptr( factory);
                let  	vtablePtr = unsafe {
                    std::mem::transmute::<
                        *const (
                            dyn Fn() -> crate::rube::coro_kernel::CoroInstance + Send + Sync
                        ),
                        ( usize, usize),
                    >( rawDyn)
                    .1
                };
                ( 4, vtablePtr)
            }
        }
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Structure-of-Arrays (SoA) SIMT Warp for homogeneous FastModule blocks.
#[derive( Clone, Debug)]
pub struct FastWarp
{
    pub _Op: KernelOp,
    pub _ModStart: u32,
    pub _Count: u32,
    pub _Mask: u64,
    pub _In1: Buff< TriggerId>,
    pub _In2: Buff< TriggerId>,
    pub _Out: Buff< TriggerId>,
}
impl FastWarp
{
    #[inline]
    pub fn	New( 
        op: KernelOp, modStart: u32, count: u32, mask: u64, in1: Buff< TriggerId>,
        in2: Buff< TriggerId>, out: Buff< TriggerId>,
    ) -> Self
    {
        Self {
            _Op: op,
            _ModStart: modStart,
            _Count: count,
            _Mask: mask,
            _In1: in1,
            _In2: in2,
            _Out: out,
        }
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Logical circuit block representation.
#[derive( Clone, Default)]
pub struct Module
{
    pub _Id: ModuleId,
    pub _Parent: ModuleId,
    pub _Name: String,
    pub _InPorts: USeg,
    pub _OutPorts: USeg,
    pub _SubModules: USeg,
    pub _Descendents: USeg,
    pub _Kernel: KernelKind,
    pub _IsSealed: bool,
}
impl Module
{
    pub fn	New( 
        id: ModuleId, parent: ModuleId, name: impl Into< String>, inPorts: USeg, outPorts: USeg,
        kernel: KernelKind,
    ) -> Self
    {
        Self {
            _Id: id,
            _Parent: parent,
            _Name: name.into(),
            _InPorts: inPorts,
            _OutPorts: outPorts,
            _SubModules: USeg::Empty(),
            _Descendents: USeg::Empty(),
            _Kernel: kernel,
            _IsSealed: false,
        }
    }
    #[inline]
    pub fn	IsContainer( &self) -> bool
    {
        self._Kernel.IsNone()
    }
}
