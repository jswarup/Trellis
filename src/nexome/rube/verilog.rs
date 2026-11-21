// verilog.rs --------------------------------------------------------------------------------------------------
use	crate::rube::layout::Layout;
use	crate::rube::module::{ KernelKind, KernelOp };
use	crate::rube::port::{ ModuleId, PortId };
use	crate::silo::{ IArr, USeg };
use	std::collections::HashSet;

//-------------------------------------------------------------------------------------------------

fn	IsReservedKeyword( name: &str) -> bool
{
    matches!(
        name,
        "always" | "and" | "assign" | "begin" | "buf" | "case" | "default" | "defparam" |
        "else" | "end" | "endcase" | "endmodule" | "for" | "function" | "if" | "initial" |
        "inout" | "input" | "integer" | "module" | "nand" | "nor" | "not" | "or" |
        "output" | "parameter" | "real" | "reg" | "supply0" | "supply1" | "task" | "time" |
        "tri" | "tri0" | "tri1" | "triand" | "trior" | "wand" | "while" | "wire" |
        "wor" | "xnor" | "xor"
    )
}

//-------------------------------------------------------------------------------------------------

pub fn	SanitizeIdentifier( name: &str) -> String
{
    if name.is_empty()
    {
        return "_unnamed".to_string();
    }
    let  	needsEscape = IsReservedKeyword( name) ||
        name.starts_with( |c: char| c.is_ascii_digit() || c == '$') ||
        name.contains( |c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$');
    if needsEscape
    {
        format!( "\\{} ", name)
    }
    else
    {
        name.to_string()
    }
}

/// Backwards compatibility alias for `SanitizeIdentifier`.
pub fn	sanitize_identifier( name: &str) -> String
{
    SanitizeIdentifier( name)
}

//-------------------------------------------------------------------------------------------------

fn	FindParentOutputConnectedTo( layout: &Layout, parent: ModuleId, driver: PortId) -> Option< PortId>
{
    let  	parentMod = layout.Module( parent);
    let  	mut found = None;
    layout.Connections().Traverse( |&( drv, sink)| {
        if drv == driver && parentMod.OutPorts().Contains( sink.Index())
        {
            found = Some( sink);
        }
    });
    found
}

//-------------------------------------------------------------------------------------------------

fn	ResolveNetName( layout: &Layout, parent: ModuleId, port: PortId) -> String
{
    let  	parentMod = layout.Module( parent);
    if parentMod.InPorts().Contains( port.Index()) || parentMod.OutPorts().Contains( port.Index())
    {
        return SanitizeIdentifier( layout.LocalPortName( port));
    }
    if port.IsOut()
    {
        if let Some( pout) = FindParentOutputConnectedTo( layout, parent, port)
        {
            return SanitizeIdentifier( layout.LocalPortName( pout));
        }
        let  	owner = layout.Port( port).Owner();
        let  	instName = SanitizeIdentifier( layout.LocalName( owner));
        let  	portName = SanitizeIdentifier( layout.LocalPortName( port));
        return format!( "{}_{}", instName, portName);
    }
    let  	driver = layout.DriverOf( port);
    if !driver.IsValid()
    {
        return "1'b0".to_string();
    }
    if parentMod.InPorts().Contains( driver.Index())
    {
        return SanitizeIdentifier( layout.LocalPortName( driver));
    }
    if let Some( pout) = FindParentOutputConnectedTo( layout, parent, driver)
    {
        return SanitizeIdentifier( layout.LocalPortName( pout));
    }
    let  	owner = layout.Port( driver).Owner();
    let  	instName = SanitizeIdentifier( layout.LocalName( owner));
    let  	portName = SanitizeIdentifier( layout.LocalPortName( driver));
    format!( "{}_{}", instName, portName)
}

//-------------------------------------------------------------------------------------------------

fn	CollectInternalWires( layout: &Layout, modId: ModuleId) -> Vec< ( String, u32)>
{
    let  	mut wires = Vec::new();
    let  	mut seen = HashSet::new();
    let  	children = layout.Children( modId);
    let  	childCount = children.Size();
    USeg::FromLen( childCount).Traverse( |cIdx| {
        let  	childId = children[cIdx];
        let  	child = layout.Module( childId);
        let  	outCount = child.OutPorts().Size();
        USeg::FromLen( outCount).Traverse( |oIdx| {
            let  	portId = layout.OutPort( childId, oIdx);
            if FindParentOutputConnectedTo( layout, modId, portId).is_none()
            {
                let  	netName = ResolveNetName( layout, modId, portId);
                if seen.insert( netName.clone())
                {
                    let  	bits = layout.Port( portId).Type().Bits();
                    wires.push( ( netName, bits));
                }
            }
        });
    });
    wires
}

//-------------------------------------------------------------------------------------------------

fn	EmitFastKernel( layout: &Layout, parent: ModuleId, childId: ModuleId, op: KernelOp) -> String
{
    let  	instName = SanitizeIdentifier( layout.LocalName( childId));
    let  	outPort = layout.OutPort( childId, 0);
    let  	outNet = ResolveNetName( layout, parent, outPort);
    let  	in1Port = layout.InPort( childId, 0);
    let  	in1Net = ResolveNetName( layout, parent, in1Port);

    match op
    {
        KernelOp::Not => format!( "    not {}({}, {});\n", instName, outNet, in1Net),
        KernelOp::Add => {
            let  	in2Net = ResolveNetName( layout, parent, layout.InPort( childId, 1));
            format!( "    assign {} = {} + {};\n", outNet, in1Net, in2Net)
        }
        KernelOp::Sub => {
            let  	in2Net = ResolveNetName( layout, parent, layout.InPort( childId, 1));
            format!( "    assign {} = {} - {};\n", outNet, in1Net, in2Net)
        }
        KernelOp::Shl => {
            let  	in2Net = ResolveNetName( layout, parent, layout.InPort( childId, 1));
            format!( "    assign {} = {} << {};\n", outNet, in1Net, in2Net)
        }
        KernelOp::Shr => {
            let  	in2Net = ResolveNetName( layout, parent, layout.InPort( childId, 1));
            format!( "    assign {} = {} >> {};\n", outNet, in1Net, in2Net)
        }
        _ => {
            let  	in2Port = layout.InPort( childId, 1);
            let  	in2Net = ResolveNetName( layout, parent, in2Port);
            let  	gateKw = match op
            {
                KernelOp::And => "and",
                KernelOp::Or => "or",
                KernelOp::Nand => "nand",
                KernelOp::Nor => "nor",
                KernelOp::Xor => "xor",
                KernelOp::Xnor => "xnor",
                _ => unreachable!(),
            };
            format!( "    {} {}({}, {}, {});\n", gateKw, instName, outNet, in1Net, in2Net)
        }
    }
}

//-------------------------------------------------------------------------------------------------

fn	EmitSubmoduleInstance( layout: &Layout, parent: ModuleId, childId: ModuleId) -> String
{
    let  	child = layout.Module( childId);
    let  	typeName = SanitizeIdentifier( child.TypeName());
    let  	instName = SanitizeIdentifier( layout.LocalName( childId));
    let  	mut s = format!( "    {} {} (\n", typeName, instName);
    let  	mut portBindings = Vec::new();

    let  	inCount = child.InPorts().Size();
    USeg::FromLen( inCount).Traverse( |i| {
        let  	portId = layout.InPort( childId, i);
        let  	portName = SanitizeIdentifier( layout.LocalPortName( portId));
        let  	netName = ResolveNetName( layout, parent, portId);
        portBindings.push( format!( "        .{}({})", portName, netName));
    });

    let  	outCount = child.OutPorts().Size();
    USeg::FromLen( outCount).Traverse( |i| {
        let  	portId = layout.OutPort( childId, i);
        let  	portName = SanitizeIdentifier( layout.LocalPortName( portId));
        let  	netName = ResolveNetName( layout, parent, portId);
        portBindings.push( format!( "        .{}({})", portName, netName));
    });

    s.push_str( &portBindings.join( ",\n"));
    s.push_str( "\n    );\n");
    s
}

//-------------------------------------------------------------------------------------------------

fn	EmitSingleModule( layout: &Layout, modId: ModuleId) -> String
{
    let  	module = layout.Module( modId);
    let  	typeName = SanitizeIdentifier( module.TypeName());
    let  	mut out = format!( "module {} (\n", typeName);
    let  	mut portDecls = Vec::new();

    let  	inCount = module.InPorts().Size();
    USeg::FromLen( inCount).Traverse( |i| {
        let  	portId = layout.InPort( modId, i);
        let  	name = SanitizeIdentifier( layout.LocalPortName( portId));
        let  	bits = layout.Port( portId).Type().Bits();
        let  	decl = if bits > 1
        {
            format!( "    input [{}:0] {}", bits - 1, name)
        }
        else
        {
            format!( "    input {}", name)
        };
        portDecls.push( decl);
    });

    let  	outCount = module.OutPorts().Size();
    USeg::FromLen( outCount).Traverse( |i| {
        let  	portId = layout.OutPort( modId, i);
        let  	name = SanitizeIdentifier( layout.LocalPortName( portId));
        let  	bits = layout.Port( portId).Type().Bits();
        let  	decl = if bits > 1
        {
            format!( "    output [{}:0] {}", bits - 1, name)
        }
        else
        {
            format!( "    output {}", name)
        };
        portDecls.push( decl);
    });

    out.push_str( &portDecls.join( ",\n"));
    out.push_str( "\n);\n");

    let  	wires = CollectInternalWires( layout, modId);
    USeg::FromLen( wires.len() as u32).Traverse( |i| {
        let  	( ref name, bits) = wires[i as usize];
        if bits > 1
        {
            out.push_str( &format!( "    wire [{}:0] {};\n", bits - 1, name));
        }
        else
        {
            out.push_str( &format!( "    wire {};\n", name));
        }
    });

    if !module.InPorts().IsEmpty() && !module.OutPorts().IsEmpty() && layout.Children( modId).IsEmpty()
    {
        // Feed-through or empty leaf module
        let  	mut connects = Vec::new();
        layout.Connections().Traverse( |&( drv, sink)| {
            if module.InPorts().Contains( drv.Index()) && module.OutPorts().Contains( sink.Index())
            {
                let  	dName = SanitizeIdentifier( layout.LocalPortName( drv));
                let  	sName = SanitizeIdentifier( layout.LocalPortName( sink));
                connects.push( format!( "    assign {} = {};\n", sName, dName));
            }
        });
        USeg::FromLen( connects.len() as u32).Traverse( |i| {
            out.push_str( &connects[i as usize]);
        });
    }

    let  	children = layout.Children( modId);
    let  	childCount = children.Size();
    USeg::FromLen( childCount).Traverse( |cIdx| {
        let  	childId = children[cIdx];
        let  	child = layout.Module( childId);
        match child.Kernel()
        {
            KernelKind::Fast( op) => {
                out.push_str( &EmitFastKernel( layout, modId, childId, *op));
            }
            KernelKind::None => {
                out.push_str( &EmitSubmoduleInstance( layout, modId, childId));
            }
            KernelKind::Coro( _) => {
                out.push_str( &EmitSubmoduleInstance( layout, modId, childId));
            }
        }
    });

    out.push_str( "endmodule\n");
    out
}

//-------------------------------------------------------------------------------------------------

fn	CollectHierarchicalModulesPostorder(
    layout: &Layout,
    modId: ModuleId,
    visited: &mut HashSet< String>,
    order: &mut Vec< ModuleId>,
)
{
    let  	children = layout.Children( modId);
    let  	childCount = children.Size();
    USeg::FromLen( childCount).Traverse( |cIdx| {
        let  	childId = children[cIdx];
        let  	child = layout.Module( childId);
        if child.Kernel().IsNone() && !layout.Children( childId).IsEmpty()
        {
            CollectHierarchicalModulesPostorder( layout, childId, visited, order);
        }
    });
    let  	typeName = layout.Module( modId).TypeName().to_string();
    if visited.insert( typeName)
    {
        order.push( modId);
    }
}

//-------------------------------------------------------------------------------------------------

pub struct VerilogExporter< 'a>
{
    _Layout: &'a Layout,
}

impl< 'a> VerilogExporter< 'a>
{
    pub fn	New( layout: &'a Layout) -> Self
    {
        Self { _Layout: layout }
    }

    pub fn	ExportModule( &self, rootId: ModuleId) -> String
    {
        let  	mut visited = HashSet::new();
        let  	mut order = Vec::new();
        CollectHierarchicalModulesPostorder( self._Layout, rootId, &mut visited, &mut order);

        let  	mut output = String::new();
        output.push_str( "// Generated by Trellis EDA - Structural Verilog Export\n\n");
        USeg::FromLen( order.len() as u32).Traverse( |i| {
            let  	mId = order[i as usize];
            output.push_str( &EmitSingleModule( self._Layout, mId));
            output.push( '\n');
        });
        output
    }

    pub fn	ExportAll( &self) -> String
    {
        let  	mut output = String::new();
        output.push_str( "// Generated by Trellis EDA - Structural Verilog Export\n\n");
        let  	mut visited = HashSet::new();
        let  	modCount = self._Layout.Modules().Size();
        USeg::FromLen( modCount).Traverse( |mIdx| {
            let  	mId = ModuleId::New( mIdx);
            let  	module = self._Layout.Module( mId);
            if !module.Parent().IsValid() && !self._Layout.Children( mId).IsEmpty()
            {
                let  	mut order = Vec::new();
                CollectHierarchicalModulesPostorder( self._Layout, mId, &mut visited, &mut order);
                USeg::FromLen( order.len() as u32).Traverse( |i| {
                    let  	ordId = order[i as usize];
                    output.push_str( &EmitSingleModule( self._Layout, ordId));
                    output.push( '\n');
                });
            }
        });
        output
    }
}

//-------------------------------------------------------------------------------------------------

/// Export a module hierarchy rooted at `rootId` to structural IEEE-1364 Verilog.
pub fn	ToVerilog( layout: &Layout, rootId: ModuleId) -> String
{
    VerilogExporter::New( layout).ExportModule( rootId)
}

/// Backwards compatibility alias for `ToVerilog`.
pub fn	to_verilog( layout: &Layout, rootId: ModuleId) -> String
{
    ToVerilog( layout, rootId)
}
