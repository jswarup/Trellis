// verilog_shard.rs -----------------------------------------------------------------------------------------
use	crate::ShardTree;
use	crate::rube::layout::Layout;
use	crate::rube::module::{ KernelKind, KernelOp };
use	crate::rube::port::{ ModuleId, PortDesc, PortId };
use	crate::shard::{ Charset, IGrammar, UInt };
use	crate::silo::{ Arr, Buff, IArr, Stash, USeg };

//-------------------------------------------------------------------------------------------------

/// Direction of a Verilog port.
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub enum VerilogPortDir
{
    Input,
    Output,
    InOut,
}

//-------------------------------------------------------------------------------------------------

/// Represents a parsed port declaration in Verilog.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogPort
{
    _Name: String,
    _Dir:  VerilogPortDir,
    _Bits: u32,
}

impl VerilogPort
{
    pub fn	New( name: impl Into< String>, dir: VerilogPortDir, bits: u32) -> Self
    {
        Self {
            _Name: name.into(),
            _Dir:  dir,
            _Bits: bits,
        }
    }

    #[inline] pub fn	Name( &self) -> &str { &self._Name }
    #[inline] pub fn	Dir( &self) -> VerilogPortDir { self._Dir }
    #[inline] pub fn	Bits( &self) -> u32 { self._Bits }
}

//-------------------------------------------------------------------------------------------------

/// Represents a parsed wire declaration in Verilog.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogWire
{
    _Name: String,
    _Bits: u32,
}

impl VerilogWire
{
    pub fn	New( name: impl Into< String>, bits: u32) -> Self
    {
        Self {
            _Name: name.into(),
            _Bits: bits,
        }
    }

    #[inline] pub fn	Name( &self) -> &str { &self._Name }
    #[inline] pub fn	Bits( &self) -> u32 { self._Bits }
}

//-------------------------------------------------------------------------------------------------

/// Connection binding between a submodule port and an internal wire/net.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogBinding
{
    _PortName: String,
    _NetName:  String,
}

impl VerilogBinding
{
    pub fn	New( portName: impl Into< String>, netName: impl Into< String>) -> Self
    {
        Self {
            _PortName: portName.into(),
            _NetName:  netName.into(),
        }
    }

    #[inline] pub fn	PortName( &self) -> &str { &self._PortName }
    #[inline] pub fn	NetName( &self) -> &str { &self._NetName }
}

//-------------------------------------------------------------------------------------------------

/// Represents a parsed instance of a submodule or primitive gate.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogInstance
{
    _TypeName: String,
    _InstName: String,
    _Bindings: Buff< VerilogBinding>,
}

impl VerilogInstance
{
    pub fn	New( typeName: impl Into< String>, instName: impl Into< String>, bindings: Buff< VerilogBinding>) -> Self
    {
        Self {
            _TypeName: typeName.into(),
            _InstName: instName.into(),
            _Bindings: bindings,
        }
    }

    #[inline] pub fn	TypeName( &self) -> &str { &self._TypeName }
    #[inline] pub fn	InstName( &self) -> &str { &self._InstName }
    #[inline] pub fn	Bindings( &self) -> Arr< '_, VerilogBinding> { self._Bindings.Arr() }
}

//-------------------------------------------------------------------------------------------------

/// Represents a continuous assign statement in Verilog: `assign lhs = rhs;`
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogAssign
{
    _Lhs:    String,
    _Rhs1:   String,
    _Rhs2:   String,
    _OpName: String,
}

impl VerilogAssign
{
    pub fn	New( lhs: impl Into< String>, rhs1: impl Into< String>, rhs2: impl Into< String>, opName: impl Into< String>) -> Self
    {
        Self {
            _Lhs:    lhs.into(),
            _Rhs1:   rhs1.into(),
            _Rhs2:   rhs2.into(),
            _OpName: opName.into(),
        }
    }

    #[inline] pub fn	Lhs( &self) -> &str { &self._Lhs }
    #[inline] pub fn	Rhs1( &self) -> &str { &self._Rhs1 }
    #[inline] pub fn	Rhs2( &self) -> &str { &self._Rhs2 }
    #[inline] pub fn	OpName( &self) -> &str { &self._OpName }
}

//-------------------------------------------------------------------------------------------------

/// Parsed structural Verilog module definition.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VerilogModule
{
    _Name:      String,
    _Ports:     Buff< VerilogPort>,
    _Wires:     Buff< VerilogWire>,
    _Instances: Buff< VerilogInstance>,
    _Assigns:   Buff< VerilogAssign>,
}

impl VerilogModule
{
    pub fn	New(
        name: impl Into< String>,
        ports: Buff< VerilogPort>,
        wires: Buff< VerilogWire>,
        instances: Buff< VerilogInstance>,
        assigns: Buff< VerilogAssign>,
    ) -> Self
    {
        Self {
            _Name:      name.into(),
            _Ports:     ports,
            _Wires:     wires,
            _Instances: instances,
            _Assigns:   assigns,
        }
    }

    #[inline] pub fn	Name( &self) -> &str { &self._Name }
    #[inline] pub fn	Ports( &self) -> Arr< '_, VerilogPort> { self._Ports.Arr() }
    #[inline] pub fn	Wires( &self) -> Arr< '_, VerilogWire> { self._Wires.Arr() }
    #[inline] pub fn	Instances( &self) -> Arr< '_, VerilogInstance> { self._Instances.Arr() }
    #[inline] pub fn	Assigns( &self) -> Arr< '_, VerilogAssign> { self._Assigns.Arr() }
}

//-------------------------------------------------------------------------------------------------

/// Parsed collection of Verilog modules ready for lowering into `rube::Layout`.
#[derive( Clone, Debug, Default, PartialEq, Eq)]
pub struct VerilogNetlist
{
    _Modules: Buff< VerilogModule>,
}

impl VerilogNetlist
{
    pub fn	New( modules: Buff< VerilogModule>) -> Self
    {
        Self { _Modules: modules }
    }

    #[inline] pub fn	Modules( &self) -> Arr< '_, VerilogModule> { self._Modules.Arr() }

    pub fn	FindModule( &self, name: &str) -> Option< &VerilogModule>
    {
        let  	mods = self._Modules.Arr();
        let  	mut foundIdx = None;
        USeg::FromLen( mods.Len()).Traverse( |i| {
            if mods.Get( i).is_some_and( |m| m.Name() == name)
            {
                foundIdx = Some( i);
            }
        });
        foundIdx.and_then( |idx| mods.Get( idx))
    }

    /// Lowers this parsed netlist into a frozen `Layout`.
    pub fn	BuildLayout( &self) -> Result< Layout, String>
    {
        let  	mut layout = Layout::New();
        let  	modCount = self._Modules.Len();
        if modCount == 0
        {
            return Err( "Cannot build layout from empty Verilog netlist".to_string());
        }

        let  	mods = self._Modules.Arr();

        // 1. Build modules in order
        USeg::FromLen( modCount).Traverse( |mIdx| {
            let  	vm = mods.Get( mIdx).unwrap();
            let  	mut inDescs = Stash::New();
            let  	mut outDescs = Stash::New();

            vm.Ports().Traverse( |p| {
                match p.Dir()
                {
                    VerilogPortDir::Input => {
                        inDescs.Push( PortDesc::Bool( p.Name()));
                    }
                    VerilogPortDir::Output => {
                        outDescs.Push( PortDesc::Bool( p.Name()));
                    }
                    VerilogPortDir::InOut => {
                        inDescs.Push( PortDesc::Bool( p.Name()));
                    }
                }
            });

            let  	inBuff = inDescs.IntoBuff();
            let  	outBuff = outDescs.IntoBuff();
            let  	parentId = ModuleId::None();
            let  	modId = layout.AddModuleWithType(
                vm.Name(),
                vm.Name(),
                parentId,
                inBuff.Arr(),
                outBuff.Arr(),
                KernelKind::None,
            );

            // 2. Instantiate child primitives and submodules
            vm.Instances().Traverse( |inst| {
                let  	instType = inst.TypeName().to_lowercase();
                let  	gateOp = match instType.as_str()
                {
                    "not" => Some( KernelOp::Not),
                    "and" => Some( KernelOp::And),
                    "or" => Some( KernelOp::Or),
                    "nand" => Some( KernelOp::Nand),
                    "nor" => Some( KernelOp::Nor),
                    "xor" => Some( KernelOp::Xor),
                    "xnor" => Some( KernelOp::Xnor),
                    "buf" => Some( KernelOp::Or),
                    _ => None,
                };

                if let Some( op) = gateOp
                {
                    if op == KernelOp::Not
                    {
                        crate::rube::gates::NotGate::WithParent( &mut layout, inst.InstName(), modId);
                    }
                    else
                    {
                        crate::rube::gates::CreateGate2( &mut layout, inst.InstName(), modId, op);
                    }
                }
            });

            // 3. Wire connections inside this module
            let  	mut netDrivers: Stash< ( String, PortId)> = Stash::New();
            let  	mut netSinks: Stash< ( String, PortId)> = Stash::New();

            let  	pMod = layout.Module( modId);
            pMod.InPorts().Traverse( |idx| {
                let  	pid = PortId::In( idx);
                let  	name = layout.LocalPortName( pid).to_string();
                netDrivers.Push( ( name, pid));
            });

            pMod.OutPorts().Traverse( |idx| {
                let  	pid = PortId::Out( idx);
                let  	name = layout.LocalPortName( pid).to_string();
                netSinks.Push( ( name, pid));
            });

            let  	children = layout.Children( modId);
            let  	childCount = children.Size();
            let  	mut instIdx = 0u32;
            vm.Instances().Traverse( |inst| {
                if instIdx < childCount
                {
                    let  	childId = children[instIdx];
                    let  	cMod = layout.Module( childId);
                    let  	bindings = inst.Bindings();

                    if let KernelKind::Fast( _) = cMod.Kernel()
                    {
                        if bindings.Len() >= 1
                        {
                            let  	outPortId = layout.OutPort( childId, 0);
                            let  	netName = bindings.Get( 0).unwrap().NetName().to_string();
                            netDrivers.Push( ( netName, outPortId));
                        }
                        if bindings.Len() >= 2
                        {
                            let  	in1PortId = layout.InPort( childId, 0);
                            let  	netName = bindings.Get( 1).unwrap().NetName().to_string();
                            netSinks.Push( ( netName, in1PortId));
                        }
                        if bindings.Len() >= 3
                        {
                            let  	in2PortId = layout.InPort( childId, 1);
                            let  	netName = bindings.Get( 2).unwrap().NetName().to_string();
                            netSinks.Push( ( netName, in2PortId));
                        }
                    }
                    instIdx += 1;
                }
            });

            netSinks.Arr().Traverse( |( sinkNet, sinkPort)| {
                let  	mut matchedPort: Option< PortId> = None;
                netDrivers.Arr().Traverse( |( drvNet, drvPort)| {
                    if drvNet == sinkNet
                    {
                        matchedPort = Some( *drvPort);
                    }
                });
                if let Some( drvPort) = matchedPort
                {
                    layout.Connect( drvPort, *sinkPort);
                }
            });
            layout.SealModule( modId);
        });

        layout.Freeze();
        Ok( layout)
    }
}

//-------------------------------------------------------------------------------------------------

/// Parser context for accumulating tokens and building `VerilogNetlist`.
pub struct VerilogParseCtx
{
    pub _Modules:         Stash< VerilogModule>,
    pub _CurrentName:     String,
    pub _CurrentPorts:    Stash< VerilogPort>,
    pub _CurrentWires:    Stash< VerilogWire>,
    pub _CurrentInsts:    Stash< VerilogInstance>,
    pub _CurrentAssigns:  Stash< VerilogAssign>,

    pub _PendingType:     String,
    pub _PendingInst:     String,
    pub _PendingBindings: Stash< VerilogBinding>,

    pub _TokenStr:        String,
    pub _TokenBits:       u32,
    pub _TokenDir:        VerilogPortDir,
    pub _BindingPort:     String,
}

impl Default for VerilogParseCtx
{
    fn	default() -> Self { Self::New() }
}

impl VerilogParseCtx
{
    pub fn	New() -> Self
    {
        Self {
            _Modules:         Stash::New(),
            _CurrentName:     String::new(),
            _CurrentPorts:    Stash::New(),
            _CurrentWires:    Stash::New(),
            _CurrentInsts:    Stash::New(),
            _CurrentAssigns:  Stash::New(),
            _PendingType:     String::new(),
            _PendingInst:     String::new(),
            _PendingBindings: Stash::New(),
            _TokenStr:        String::new(),
            _TokenBits:       1,
            _TokenDir:        VerilogPortDir::Input,
            _BindingPort:     String::new(),
        }
    }

    pub fn	SetTokenStr( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        let  	s = std::str::from_utf8( slice).unwrap_or( "").trim();
        self._TokenStr = s.to_string();
        true
    }

    pub fn	SetModuleName( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        let  	s = std::str::from_utf8( slice).unwrap_or( "").trim();
        self._CurrentName = s.to_string();
        true
    }

    pub fn	AddPort( &mut self) -> bool
    {
        if !self._TokenStr.is_empty()
        {
            let  	mut exists = false;
            self._CurrentPorts.Arr().Traverse( |p| {
                if p.Name() == self._TokenStr
                {
                    exists = true;
                }
            });
            if !exists
            {
                self._CurrentPorts.Push( VerilogPort::New( &self._TokenStr, self._TokenDir, self._TokenBits));
            }
            self._TokenStr.clear();
            self._TokenBits = 1;
        }
        true
    }

    pub fn	AddWire( &mut self) -> bool
    {
        if !self._TokenStr.is_empty()
        {
            self._CurrentWires.Push( VerilogWire::New( &self._TokenStr, self._TokenBits));
            self._TokenStr.clear();
            self._TokenBits = 1;
        }
        true
    }

    pub fn	SetInstType( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        self._PendingType = std::str::from_utf8( slice).unwrap_or( "").trim().to_string();
        true
    }

    pub fn	SetInstName( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        self._PendingInst = std::str::from_utf8( slice).unwrap_or( "").trim().to_string();
        true
    }

    pub fn	AddPositionalBinding( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        let  	net = std::str::from_utf8( slice).unwrap_or( "").trim();
        if !net.is_empty()
        {
            self._PendingBindings.Push( VerilogBinding::New( "", net));
        }
        true
    }

    pub fn	SetNamedBindingPort( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        self._BindingPort = std::str::from_utf8( slice).unwrap_or( "").trim().to_string();
        true
    }

    pub fn	AddNamedBindingNet( &mut self, arr: Arr< '_, u8>) -> bool
    {
        let  	slice: &[u8] = arr.into();
        let  	net = std::str::from_utf8( slice).unwrap_or( "").trim();
        self._PendingBindings.Push( VerilogBinding::New( &self._BindingPort, net));
        self._BindingPort.clear();
        true
    }

    pub fn	CommitInstance( &mut self) -> bool
    {
        if !self._PendingType.is_empty()
        {
            let  	bindings = std::mem::take( &mut self._PendingBindings).IntoBuff();
            self._CurrentInsts.Push( VerilogInstance::New( &self._PendingType, &self._PendingInst, bindings));
            self._PendingType.clear();
            self._PendingInst.clear();
        }
        true
    }

    pub fn	CommitModule( &mut self) -> bool
    {
        if !self._CurrentName.is_empty()
        {
            let  	ports = std::mem::take( &mut self._CurrentPorts).IntoBuff();
            let  	wires = std::mem::take( &mut self._CurrentWires).IntoBuff();
            let  	insts = std::mem::take( &mut self._CurrentInsts).IntoBuff();
            let  	assigns = std::mem::take( &mut self._CurrentAssigns).IntoBuff();
            self._Modules.Push( VerilogModule::New( &self._CurrentName, ports, wires, insts, assigns));
            self._CurrentName.clear();
        }
        true
    }

    pub fn	IntoNetlist( self) -> VerilogNetlist
    {
        VerilogNetlist::New( self._Modules.IntoBuff())
    }
}

//-------------------------------------------------------------------------------------------------

/// Stateless Shard grammar for parsing structural IEEE-1364 Verilog netlists.
pub struct VerilogGrammar;

impl IGrammar< VerilogParseCtx> for VerilogGrammar
{
    fn	Match( &self, parser: &mut crate::shard::Parser, ctx: &mut VerilogParseCtx) -> bool
    {
        let  	notEol = Charset::EndLine().Negative();
        let  	notStar = Charset::from( &b"*"[..]).Negative();
        let  	notSlash = Charset::from( &b"/"[..]).Negative();
        let  	notSemi = Charset::from( &b";\r\n"[..]).Negative();
        let  	identChar = Charset::from( &b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_$"[..]);

        let  	comment = ShardTree!( 
            ( "//" < *notEol < ?( ( ?'\r' < '\n' ) | '\r' ) )
            | ( "/*" < *( notStar | ( "*" < notSlash ) ) < "*/" )
        );

        let  	spaceOrComment = ShardTree!( +( [ " \t\r\n" ] | comment ) );
        let  	optSpace = ShardTree!( *( [ " \t\r\n" ] | comment ) );

        let  	ident = ShardTree!( +identChar );

        let  	portDecl = ShardTree!( 
            ( ( "input"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::Input; true } ]
                | "output"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::Output; true } ]
                | "inout"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::InOut; true } ] )
              < optSpace
              < ?( "[" < optSpace < UInt[ |arr, ctx: &mut VerilogParseCtx| {
                    let  	slice: &[u8] = arr.into();
                    let  	msb = std::str::from_utf8( slice).unwrap_or( "0").parse::< u32>().unwrap_or( 0);
                    ctx._TokenBits = msb + 1;
                    true
                } ] < optSpace < ":" < optSpace < UInt < optSpace < "]" )
              < optSpace
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetTokenStr( arr) ]
              < optSpace
              < ?( "," | ";" ) )[ |_arr, ctx: &mut VerilogParseCtx| ctx.AddPort() ]
        );

        let  	wireDecl = ShardTree!( 
            ( "wire"
              < optSpace
              < ?( "[" < optSpace < UInt[ |arr, ctx: &mut VerilogParseCtx| {
                    let  	slice: &[u8] = arr.into();
                    let  	msb = std::str::from_utf8( slice).unwrap_or( "0").parse::< u32>().unwrap_or( 0);
                    ctx._TokenBits = msb + 1;
                    true
                } ] < optSpace < ":" < optSpace < UInt < optSpace < "]" )
              < optSpace
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetTokenStr( arr) ]
              < optSpace
              < ";" )[ |_arr, ctx: &mut VerilogParseCtx| ctx.AddWire() ]
        );

        let  	bindingItem = ShardTree!( 
            ( "." < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetNamedBindingPort( arr) ]
              < optSpace < "(" < optSpace
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.AddNamedBindingNet( arr) ]
              < optSpace < ")" )
            | ( ident[ |arr, ctx: &mut VerilogParseCtx| ctx.AddPositionalBinding( arr) ] )
        );

        let  	instanceStmt = ShardTree!( 
            ( ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetInstType( arr) ]
              < spaceOrComment
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetInstName( arr) ]
              < optSpace < "(" < optSpace
              < *( optSpace < bindingItem < ?( optSpace < "," ) )
              < optSpace < ")" < optSpace < ";" )[ |_arr, ctx: &mut VerilogParseCtx| ctx.CommitInstance() ]
        );

        let  	moduleItem = ShardTree!( 
            portDecl
            | wireDecl
            | instanceStmt
            | ( "assign" < *( notSemi ) < ";" )
        );

        let  	headerPortDecl = ShardTree!( 
            ( ( "input"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::Input; true } ]
                | "output"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::Output; true } ]
                | "inout"[ |_arr, ctx: &mut VerilogParseCtx| { ctx._TokenDir = VerilogPortDir::InOut; true } ] )
              < optSpace
              < ?( "[" < optSpace < UInt[ |arr, ctx: &mut VerilogParseCtx| {
                    let  	slice: &[u8] = arr.into();
                    let  	msb = std::str::from_utf8( slice).unwrap_or( "0").parse::< u32>().unwrap_or( 0);
                    ctx._TokenBits = msb + 1;
                    true
                } ] < optSpace < ":" < optSpace < UInt < optSpace < "]" )
              < optSpace
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetTokenStr( arr) ] )[ |_arr, ctx: &mut VerilogParseCtx| ctx.AddPort() ]
        );

        let  	portList = ShardTree!( 
            "(" < optSpace
            < *( optSpace < ( headerPortDecl | ident ) < ?( optSpace < "," ) )
            < optSpace < ")"
        );

        let  	moduleDef = ShardTree!( 
            ( "module"
              < spaceOrComment
              < ident[ |arr, ctx: &mut VerilogParseCtx| ctx.SetModuleName( arr) ]
              < optSpace
              < ?( portList )
              < optSpace < ";"
              < *( optSpace < moduleItem )
              < optSpace
              < "endmodule" )[ |_arr, ctx: &mut VerilogParseCtx| ctx.CommitModule() ]
        );

        let  	verilogFile = ShardTree!( 
            optSpace
            < *( moduleDef < optSpace )
        );

        verilogFile.Match( parser, ctx)
    }
}

//-------------------------------------------------------------------------------------------------

/// Parses a structural Verilog netlist from a string slice.
pub fn	ParseVerilogNetlist( input: &str) -> Result< VerilogNetlist, String>
{
    let  	mut ctx = VerilogParseCtx::New();
    VerilogGrammar
        .ParseStr( input, &mut ctx)
        .map( |_| ctx.IntoNetlist())
        .map_err( |e| e.to_string())
}

//-------------------------------------------------------------------------------------------------
