// rube_resource.rs -------------------------------------------------------------------------------
//! Typed schematic and recorded simulation for the built-in Dadda example.

use crate::rube::{DaddaMult4x4, Layout, SimEngine, VcdWriter};
use crate::rube::circuit_xplr::RubeCircuit;
use crate::silo::Arr;

//-------------------------------------------------------------------------------------------------

pub const DADDA_EXAMPLE_URI: &str = "example://Rube/DaddaMultiplier4x4";
pub const DADDA_STIMULUS: [(u8, u8); 4] = [(0, 0), (3, 5), (7, 9), (15, 15)];

#[derive(Clone, Debug)]
pub enum DaddaRunError
{
    Cancelled,
    Failed(String),
}

fn BuildDadda() -> (Layout, DaddaMult4x4)
{
    let mut layout = Layout::New();
    let circuit = DaddaMult4x4::New(&mut layout, "Dadda4x4");
    layout.Freeze();
    return (layout, circuit);
}

pub fn OpenDaddaSchematic() -> RubeCircuit
{
    let (layout, circuit) = BuildDadda();
    let id = layout.Port(circuit.InA(0)).Owner();
    return layout.IntoCircuitWithPath(id, DADDA_EXAMPLE_URI);
}

pub fn RunDadda(stimulus: Arr<'_, (u8, u8)>, cancelled: impl Fn() -> bool)
                -> Result<String, DaddaRunError>
{
    if stimulus.IsEmpty() || stimulus.Size() > 256 {
        return Err(DaddaRunError::Failed("Provide 1 to 256 Dadda input pairs".into()));
    }
    if cancelled() {
        return Err(DaddaRunError::Cancelled);
    }
    let (layout, circuit) = BuildDadda();
    let mut engine = SimEngine::Create(&layout);
    let writer = VcdWriter::New(&layout, &engine);
    let mut trace = String::new();
    writer.WriteHeader(&layout, &engine, &mut trace);
    let mut result = Ok(());
    stimulus.USeg().Span(|index| {
        let (a, b) = stimulus[index];
        if a > 15 || b > 15 {
            result = Err(DaddaRunError::Failed(format!("Input pair {index} exceeds four bits")));
            return false;
        }
        circuit.SetA(&mut engine, a);
        circuit.SetB(&mut engine, b);
        let mut cycles = 0;
        while cycles < 20 {
            if cancelled() {
                result = Err(DaddaRunError::Cancelled);
                return false;
            }
            engine.StepCycle();
            writer.DumpCycle(&engine, &mut trace);
            if !engine.Triggers().HasAnyEdge() {
                return true;
            }
            cycles += 1;
        }
        result = Err(DaddaRunError::Failed(format!("Input pair {index} did not settle")));
        return false;
    });
    result?;
    return Ok(trace);
}

//-------------------------------------------------------------------------------------------------
