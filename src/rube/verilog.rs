// verilog.rs --------------------------------------------------------------------------------------------------
use crate::rube::layout::Layout;
use crate::rube::module::{KernelKind, KernelOp};
use crate::rube::port::{ModuleId, PortId};
use crate::silo::IArr;
use std::collections::HashSet;

//-------------------------------------------------------------------------------------------------

fn is_reserved_keyword(name: &str) -> bool
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

pub fn sanitize_identifier(name: &str) -> String
{
    if name.is_empty() {
        return "_unnamed".to_string();
    }
    let needs_escape = is_reserved_keyword(name) ||
        name.starts_with(|c: char| c.is_ascii_digit() || c == '$') ||
        name.contains(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$');
    if needs_escape {
        format!("\\{} ", name)
    } else {
        name.to_string()
    }
}

//-------------------------------------------------------------------------------------------------

fn find_parent_output_connected_to(layout: &Layout, parent: ModuleId, driver: PortId) -> Option<PortId>
{
    let parent_mod = layout.Module(parent);
    let mut found = None;
    layout.Connections().Traverse(|&(drv, sink)| {
        if drv == driver && parent_mod.OutPorts().Contains(sink.Index()) {
            found = Some(sink);
        }
    });
    found
}

//-------------------------------------------------------------------------------------------------

fn resolve_net_name(layout: &Layout, parent: ModuleId, port: PortId) -> String
{
    let parent_mod = layout.Module(parent);
    if parent_mod.InPorts().Contains(port.Index()) || parent_mod.OutPorts().Contains(port.Index()) {
        return sanitize_identifier(layout.LocalPortName(port));
    }
    if port.IsOut() {
        if let Some(pout) = find_parent_output_connected_to(layout, parent, port) {
            return sanitize_identifier(layout.LocalPortName(pout));
        }
        let owner = layout.Port(port).Owner();
        let inst_name = sanitize_identifier(layout.LocalName(owner));
        let port_name = sanitize_identifier(layout.LocalPortName(port));
        return format!("{}_{}", inst_name, port_name);
    }
    let driver = layout.DriverOf(port);
    if !driver.IsValid() {
        return "1'b0".to_string();
    }
    if parent_mod.InPorts().Contains(driver.Index()) {
        return sanitize_identifier(layout.LocalPortName(driver));
    }
    if let Some(pout) = find_parent_output_connected_to(layout, parent, driver) {
        return sanitize_identifier(layout.LocalPortName(pout));
    }
    let owner = layout.Port(driver).Owner();
    let inst_name = sanitize_identifier(layout.LocalName(owner));
    let port_name = sanitize_identifier(layout.LocalPortName(driver));
    format!("{}_{}", inst_name, port_name)
}

//-------------------------------------------------------------------------------------------------

fn collect_internal_wires(layout: &Layout, mod_id: ModuleId) -> Vec<(String, u32)>
{
    let mut wires = Vec::new();
    let mut seen = HashSet::new();
    let children = layout.Children(mod_id);
    let child_count = children.Size();
    for c_idx in 0..child_count {
        let child_id = children[c_idx];
        let child = layout.Module(child_id);
        let out_count = child.OutPorts().Size();
        for o_idx in 0..out_count {
            let port_id = layout.OutPort(child_id, o_idx);
            if find_parent_output_connected_to(layout, mod_id, port_id).is_none() {
                let net_name = resolve_net_name(layout, mod_id, port_id);
                if seen.insert(net_name.clone()) {
                    let bits = layout.Port(port_id).Type().Bits();
                    wires.push((net_name, bits));
                }
            }
        }
    }
    wires
}

//-------------------------------------------------------------------------------------------------

fn emit_fast_kernel(layout: &Layout, parent: ModuleId, child_id: ModuleId, op: KernelOp) -> String
{
    let inst_name = sanitize_identifier(layout.LocalName(child_id));
    let out_port = layout.OutPort(child_id, 0);
    let out_net = resolve_net_name(layout, parent, out_port);
    let in1_port = layout.InPort(child_id, 0);
    let in1_net = resolve_net_name(layout, parent, in1_port);

    match op {
        KernelOp::Not => format!("    not {}({}, {});\n", inst_name, out_net, in1_net),
        KernelOp::Add => {
            let in2_net = resolve_net_name(layout, parent, layout.InPort(child_id, 1));
            format!("    assign {} = {} + {};\n", out_net, in1_net, in2_net)
        }
        KernelOp::Sub => {
            let in2_net = resolve_net_name(layout, parent, layout.InPort(child_id, 1));
            format!("    assign {} = {} - {};\n", out_net, in1_net, in2_net)
        }
        KernelOp::Shl => {
            let in2_net = resolve_net_name(layout, parent, layout.InPort(child_id, 1));
            format!("    assign {} = {} << {};\n", out_net, in1_net, in2_net)
        }
        KernelOp::Shr => {
            let in2_net = resolve_net_name(layout, parent, layout.InPort(child_id, 1));
            format!("    assign {} = {} >> {};\n", out_net, in1_net, in2_net)
        }
        _ => {
            let in2_port = layout.InPort(child_id, 1);
            let in2_net = resolve_net_name(layout, parent, in2_port);
            let gate_kw = match op {
                KernelOp::And => "and",
                KernelOp::Or => "or",
                KernelOp::Nand => "nand",
                KernelOp::Nor => "nor",
                KernelOp::Xor => "xor",
                KernelOp::Xnor => "xnor",
                _ => unreachable!(),
            };
            format!("    {} {}({}, {}, {});\n", gate_kw, inst_name, out_net, in1_net, in2_net)
        }
    }
}

//-------------------------------------------------------------------------------------------------

fn emit_submodule_instance(layout: &Layout, parent: ModuleId, child_id: ModuleId) -> String
{
    let child = layout.Module(child_id);
    let type_name = sanitize_identifier(child.TypeName());
    let inst_name = sanitize_identifier(layout.LocalName(child_id));
    let mut s = format!("    {} {} (\n", type_name, inst_name);
    let mut port_bindings = Vec::new();

    let in_count = child.InPorts().Size();
    for i in 0..in_count {
        let port_id = layout.InPort(child_id, i);
        let port_name = sanitize_identifier(layout.LocalPortName(port_id));
        let net_name = resolve_net_name(layout, parent, port_id);
        port_bindings.push(format!("        .{}({})", port_name, net_name));
    }

    let out_count = child.OutPorts().Size();
    for i in 0..out_count {
        let port_id = layout.OutPort(child_id, i);
        let port_name = sanitize_identifier(layout.LocalPortName(port_id));
        let net_name = resolve_net_name(layout, parent, port_id);
        port_bindings.push(format!("        .{}({})", port_name, net_name));
    }

    s.push_str(&port_bindings.join(",\n"));
    s.push_str("\n    );\n");
    s
}

//-------------------------------------------------------------------------------------------------

fn emit_single_module(layout: &Layout, mod_id: ModuleId) -> String
{
    let module = layout.Module(mod_id);
    let type_name = sanitize_identifier(module.TypeName());
    let mut out = format!("module {} (\n", type_name);
    let mut port_decls = Vec::new();

    let in_count = module.InPorts().Size();
    for i in 0..in_count {
        let port_id = layout.InPort(mod_id, i);
        let name = sanitize_identifier(layout.LocalPortName(port_id));
        let bits = layout.Port(port_id).Type().Bits();
        let decl = if bits > 1 {
            format!("    input [{}:0] {}", bits - 1, name)
        } else {
            format!("    input {}", name)
        };
        port_decls.push(decl);
    }

    let out_count = module.OutPorts().Size();
    for i in 0..out_count {
        let port_id = layout.OutPort(mod_id, i);
        let name = sanitize_identifier(layout.LocalPortName(port_id));
        let bits = layout.Port(port_id).Type().Bits();
        let decl = if bits > 1 {
            format!("    output [{}:0] {}", bits - 1, name)
        } else {
            format!("    output {}", name)
        };
        port_decls.push(decl);
    }

    out.push_str(&port_decls.join(",\n"));
    out.push_str("\n);\n");

    let wires = collect_internal_wires(layout, mod_id);
    for (name, bits) in wires {
        if bits > 1 {
            out.push_str(&format!("    wire [{}:0] {};\n", bits - 1, name));
        } else {
            out.push_str(&format!("    wire {};\n", name));
        }
    }
    if !module.InPorts().IsEmpty() && !module.OutPorts().IsEmpty() && layout.Children(mod_id).IsEmpty() {
        // Feed-through or empty leaf module
        let mut connects = Vec::new();
        layout.Connections().Traverse(|&(drv, sink)| {
            if module.InPorts().Contains(drv.Index()) && module.OutPorts().Contains(sink.Index()) {
                let d_name = sanitize_identifier(layout.LocalPortName(drv));
                let s_name = sanitize_identifier(layout.LocalPortName(sink));
                connects.push(format!("    assign {} = {};\n", s_name, d_name));
            }
        });
        for c in connects {
            out.push_str(&c);
        }
    }

    let children = layout.Children(mod_id);
    let child_count = children.Size();
    for c_idx in 0..child_count {
        let child_id = children[c_idx];
        let child = layout.Module(child_id);
        match child.Kernel() {
            KernelKind::Fast(op) => {
                out.push_str(&emit_fast_kernel(layout, mod_id, child_id, *op));
            }
            KernelKind::None => {
                out.push_str(&emit_submodule_instance(layout, mod_id, child_id));
            }
            KernelKind::Coro(_) => {
                out.push_str(&emit_submodule_instance(layout, mod_id, child_id));
            }
        }
    }

    out.push_str("endmodule\n");
    out
}

//-------------------------------------------------------------------------------------------------

fn collect_hierarchical_modules_postorder(
    layout: &Layout,
    mod_id: ModuleId,
    visited: &mut HashSet<String>,
    order: &mut Vec<ModuleId>,
)
{
    let children = layout.Children(mod_id);
    let child_count = children.Size();
    for c_idx in 0..child_count {
        let child_id = children[c_idx];
        let child = layout.Module(child_id);
        if child.Kernel().IsNone() && !layout.Children(child_id).IsEmpty() {
            collect_hierarchical_modules_postorder(layout, child_id, visited, order);
        }
    }
    let type_name = layout.Module(mod_id).TypeName().to_string();
    if visited.insert(type_name) {
        order.push(mod_id);
    }
}

//-------------------------------------------------------------------------------------------------

pub struct VerilogExporter<'a>
{
    _layout: &'a Layout,
}

impl<'a> VerilogExporter<'a>
{
    pub fn New(layout: &'a Layout) -> Self
    {
        Self { _layout: layout }
    }

    pub fn ExportModule(&self, root_id: ModuleId) -> String
    {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        collect_hierarchical_modules_postorder(self._layout, root_id, &mut visited, &mut order);

        let mut output = String::new();
        output.push_str("// Generated by Trellis EDA - Structural Verilog Export\n\n");
        for &m_id in &order {
            output.push_str(&emit_single_module(self._layout, m_id));
            output.push('\n');
        }
        output
    }

    pub fn ExportAll(&self) -> String
    {
        let mut output = String::new();
        output.push_str("// Generated by Trellis EDA - Structural Verilog Export\n\n");
        let mut visited = HashSet::new();
        let mod_count = self._layout.Modules().Size();
        for m_idx in 0..mod_count {
            let m_id = ModuleId::New(m_idx);
            let module = self._layout.Module(m_id);
            if !module.Parent().IsValid() && !self._layout.Children(m_id).IsEmpty() {
                let mut order = Vec::new();
                collect_hierarchical_modules_postorder(self._layout, m_id, &mut visited, &mut order);
                for &ord_id in &order {
                    output.push_str(&emit_single_module(self._layout, ord_id));
                    output.push('\n');
                }
            }
        }
        output
    }
}

//-------------------------------------------------------------------------------------------------
