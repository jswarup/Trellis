//-- dadda.rs ------------------------------------------------------------------------------------------------------
use crate::rube::adder::{ FullAdder, HalfAdder };
use crate::rube::engine::SimEngine;
use crate::rube::gates::AndGate;
use crate::rube::layout::Layout;
use crate::rube::module::KernelKind;
use crate::rube::port::{ ModuleId, PortDesc, PortId };
use crate::silo::USeg;

//------------------------------------------------------------------------------------------------------------------
// Structural 4x4 Dadda Multiplier (Reduction Tree + Carry Propagate Adder).
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DaddaMult4x4
{
    _Id:      ModuleId,
    _A:       [PortId; 4],
    _B:       [PortId; 4],
    _Product: [PortId; 8],
}

impl DaddaMult4x4
{
    #[inline]
    pub fn  New( layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent( layout, name, ModuleId::None())
    }

    pub fn  WithParent( layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let     inDescs = [
            PortDesc::Bool( "a0"),
            PortDesc::Bool( "a1"),
            PortDesc::Bool( "a2"),
            PortDesc::Bool( "a3"),
            PortDesc::Bool( "b0"),
            PortDesc::Bool( "b1"),
            PortDesc::Bool( "b2"),
            PortDesc::Bool( "b3"),
        ];
        let     outDescs = [
            PortDesc::Bool( "p0"),
            PortDesc::Bool( "p1"),
            PortDesc::Bool( "p2"),
            PortDesc::Bool( "p3"),
            PortDesc::Bool( "p4"),
            PortDesc::Bool( "p5"),
            PortDesc::Bool( "p6"),
            PortDesc::Bool( "p7"),
        ];
        let     id = layout.AddModule( name, parent, &inDescs[..], &outDescs[..], KernelKind::None);

        let     aPorts = [
            layout.InPort( id, 0),
            layout.InPort( id, 1),
            layout.InPort( id, 2),
            layout.InPort( id, 3),
        ];
        let     bPorts = [
            layout.InPort( id, 4),
            layout.InPort( id, 5),
            layout.InPort( id, 6),
            layout.InPort( id, 7),
        ];
        let     prodPorts = [
            layout.OutPort( id, 0),
            layout.OutPort( id, 1),
            layout.OutPort( id, 2),
            layout.OutPort( id, 3),
            layout.OutPort( id, 4),
            layout.OutPort( id, 5),
            layout.OutPort( id, 6),
            layout.OutPort( id, 7),
        ];

        let     nameStr = if name.is_empty() { "Dadda4x4" } else { name };

        // Step 1: Generate 16 Partial Products via AND gates (p_row_col = B[row] & A[col])
        let     mut p = [[PortId::Invalid(); 4]; 4];
        USeg::FromLen( 4).Traverse( |row| {
            USeg::FromLen( 4).Traverse( |col| {
                let     gateName = format!( "{}.P{}{}", nameStr, row, col);
                let     andGate = AndGate::WithParent( layout, &gateName, id);
                layout.Connect( aPorts[col as usize], andGate.In1());
                layout.Connect( bPorts[row as usize], andGate.In2());
                p[row as usize][col as usize] = andGate.Out();
            });
        });

        // Step 2: Reduction Stage 1 (Target height d2 = 3).
        // Only Column 3 has 4 elements (p03, p12, p21, p30). Reduce using 1 Half Adder on p03 and p12.
        let     haCol3Stg1Name = format!( "{}.HA_Col3_Stg1", nameStr);
        let     haCol3Stg1 = HalfAdder::WithParent( layout, &haCol3Stg1Name, id);
        layout.Connect( p[0][3], haCol3Stg1.In1());
        layout.Connect( p[1][2], haCol3Stg1.In2());
        let     s1C3 = haCol3Stg1.Sum();
        let     c1C4 = haCol3Stg1.Carry();

        // Step 3: Reduction Stage 2 (Target height d1 = 2).
        // Column 2 (height 3: p02, p11, p20) -> 1 Full Adder
        let     faCol2Stg2Name = format!( "{}.FA_Col2_Stg2", nameStr);
        let     faCol2Stg2 = FullAdder::WithParent( layout, &faCol2Stg2Name, id);
        layout.Connect( p[0][2], faCol2Stg2.In1());
        layout.Connect( p[1][1], faCol2Stg2.In2());
        layout.Connect( p[2][0], faCol2Stg2.CIn());
        let     s2C2 = faCol2Stg2.Sum();
        let     c2C3 = faCol2Stg2.Carry();

        // Column 3 (height 3 from Stage 1: s1C3, p21, p30) -> 1 Full Adder
        let     faCol3Stg2Name = format!( "{}.FA_Col3_Stg2", nameStr);
        let     faCol3Stg2 = FullAdder::WithParent( layout, &faCol3Stg2Name, id);
        layout.Connect( s1C3, faCol3Stg2.In1());
        layout.Connect( p[2][1], faCol3Stg2.In2());
        layout.Connect( p[3][0], faCol3Stg2.CIn());
        let     s2C3 = faCol3Stg2.Sum();
        let     c2C4 = faCol3Stg2.Carry();

        // Column 4 (height 4: p13, p22, p31 + c1C4 from stage 1) -> 1 FA + 1 HA
        let     faCol4Stg2Name = format!( "{}.FA_Col4_Stg2", nameStr);
        let     faCol4Stg2 = FullAdder::WithParent( layout, &faCol4Stg2Name, id);
        layout.Connect( p[1][3], faCol4Stg2.In1());
        layout.Connect( p[2][2], faCol4Stg2.In2());
        layout.Connect( p[3][1], faCol4Stg2.CIn());
        let     s2C4Fa = faCol4Stg2.Sum();
        let     c2C5Fa = faCol4Stg2.Carry();

        let     haCol4Stg2Name = format!( "{}.HA_Col4_Stg2", nameStr);
        let     haCol4Stg2 = HalfAdder::WithParent( layout, &haCol4Stg2Name, id);
        layout.Connect( s2C4Fa, haCol4Stg2.In1());
        layout.Connect( c1C4, haCol4Stg2.In2());
        let     s2C4Final = haCol4Stg2.Sum();
        let     c2C5Ha = haCol4Stg2.Carry();

        // Column 5 (height 4: p23, p32 + c2C5Fa, c2C5Ha) -> 1 Full Adder reducing to height 2
        let     faCol5Stg2Name = format!( "{}.FA_Col5_Stg2", nameStr);
        let     faCol5Stg2 = FullAdder::WithParent( layout, &faCol5Stg2Name, id);
        layout.Connect( p[2][3], faCol5Stg2.In1());
        layout.Connect( p[3][2], faCol5Stg2.In2());
        layout.Connect( c2C5Fa, faCol5Stg2.CIn());
        let     s2C5 = faCol5Stg2.Sum();
        let     c2C6 = faCol5Stg2.Carry();

        // Step 4: Final Addition Stage via Carry Propagate Adder (CPA)
        // Bit 0:
        layout.Connect( p[0][0], prodPorts[0]);

        // Bit 1: HA(p01, p10)
        let     haCpa1Name = format!( "{}.HA_CPA1", nameStr);
        let     haCpa1 = HalfAdder::WithParent( layout, &haCpa1Name, id);
        layout.Connect( p[0][1], haCpa1.In1());
        layout.Connect( p[1][0], haCpa1.In2());
        layout.Connect( haCpa1.Sum(), prodPorts[1]);
        let     cpaC1 = haCpa1.Carry();

        // Bit 2: HA(s2C2, cpaC1)
        let     haCpa2Name = format!( "{}.HA_CPA2", nameStr);
        let     haCpa2 = HalfAdder::WithParent( layout, &haCpa2Name, id);
        layout.Connect( s2C2, haCpa2.In1());
        layout.Connect( cpaC1, haCpa2.In2());
        layout.Connect( haCpa2.Sum(), prodPorts[2]);
        let     cpaC2 = haCpa2.Carry();

        // Bit 3: FA(s2C3, c2C3, cpaC2)
        let     faCpa3Name = format!( "{}.FA_CPA3", nameStr);
        let     faCpa3 = FullAdder::WithParent( layout, &faCpa3Name, id);
        layout.Connect( s2C3, faCpa3.In1());
        layout.Connect( c2C3, faCpa3.In2());
        layout.Connect( cpaC2, faCpa3.CIn());
        layout.Connect( faCpa3.Sum(), prodPorts[3]);
        let     cpaC3 = faCpa3.Carry();

        // Bit 4: FA(s2C4Final, c2C4, cpaC3)
        let     faCpa4Name = format!( "{}.FA_CPA4", nameStr);
        let     faCpa4 = FullAdder::WithParent( layout, &faCpa4Name, id);
        layout.Connect( s2C4Final, faCpa4.In1());
        layout.Connect( c2C4, faCpa4.In2());
        layout.Connect( cpaC3, faCpa4.CIn());
        layout.Connect( faCpa4.Sum(), prodPorts[4]);
        let     cpaC4 = faCpa4.Carry();

        // Bit 5: FA(s2C5, c2C5Ha, cpaC4)
        let     faCpa5Name = format!( "{}.FA_CPA5", nameStr);
        let     faCpa5 = FullAdder::WithParent( layout, &faCpa5Name, id);
        layout.Connect( s2C5, faCpa5.In1());
        layout.Connect( c2C5Ha, faCpa5.In2());
        layout.Connect( cpaC4, faCpa5.CIn());
        layout.Connect( faCpa5.Sum(), prodPorts[5]);
        let     cpaC5 = faCpa5.Carry();

        // Bit 6: FA(p33, c2C6, cpaC5)
        let     faCpa6Name = format!( "{}.FA_CPA6", nameStr);
        let     faCpa6 = FullAdder::WithParent( layout, &faCpa6Name, id);
        layout.Connect( p[3][3], faCpa6.In1());
        layout.Connect( c2C6, faCpa6.In2());
        layout.Connect( cpaC5, faCpa6.CIn());
        layout.Connect( faCpa6.Sum(), prodPorts[6]);
        let     cpaC6 = faCpa6.Carry();

        // Bit 7: cpaC6
        layout.Connect( cpaC6, prodPorts[7]);

        layout.SealModule( id);

        Self {
            _Id:      id,
            _A:       aPorts,
            _B:       bPorts,
            _Product: prodPorts,
        }
    }

    #[inline]
    pub const fn    Id( &self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn    InA( &self, idx: usize) -> PortId
    {
        self._A[idx]
    }

    #[inline]
    pub const fn    InB( &self, idx: usize) -> PortId
    {
        self._B[idx]
    }

    #[inline]
    pub const fn    Out( &self, idx: usize) -> PortId
    {
        self._Product[idx]
    }

    /// Sets 4-bit operand A (bits 0..3).
    pub fn  SetA( &self, engine: &mut SimEngine, val: u8)
    {
        USeg::FromLen( 4).Traverse( |i| {
            let     bit = ( ( val >> i) & 1) != 0;
            engine.SetBool( self._A[i as usize], bit);
        });
    }

    /// Sets 4-bit operand B (bits 0..3).
    pub fn  SetB( &self, engine: &mut SimEngine, val: u8)
    {
        USeg::FromLen( 4).Traverse( |i| {
            let     bit = ( ( val >> i) & 1) != 0;
            engine.SetBool( self._B[i as usize], bit);
        });
    }

    /// Reads settled 8-bit Product (bits 0..7).
    pub fn  GetProduct( &self, engine: &SimEngine) -> u8
    {
        let     mut res = 0u8;
        USeg::FromLen( 8).Traverse( |i| {
            if engine.GetBool( self._Product[i as usize]) {
                res |= 1 << i;
            }
        });
        res
    }
}

//-------------------------------------------------------------------------------------------------
