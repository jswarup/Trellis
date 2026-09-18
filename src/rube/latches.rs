//-- latches.rs ----------------------------------------------------------------------------------------------------

use crate::rube::engine::SimEngine;
use crate::rube::gates::{NandGate, NotGate};
use crate::rube::layout::Layout;
use crate::rube::module::KernelKind;
use crate::rube::port::{ModuleId, PortDesc, PortId};

//------------------------------------------------------------------------------------------------------------------
// Asynchronous RS Latch (Cross-coupled NAND gates).
// Modeled directly from Trellis `latches.h`.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RSLatch
{
    pub _Id:    ModuleId,
    pub _Nand1: NandGate,
    pub _Nand2: NandGate,
    pub _S:     PortId,
    pub _R:     PortId,
    pub _Q:     PortId,
    pub _Q1:    PortId,
}

impl RSLatch
{
    #[inline]
    pub fn New(layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent(layout, name, ModuleId::None())
    }

    pub fn WithParent(layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let inDescs = [PortDesc::Bool("S"), PortDesc::Bool("R")];
        let outDescs = [PortDesc::Bool("Q"), PortDesc::Bool("Q1")];

        let id = layout.AddModule(
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::None,
        );

        let s = layout.InPort(id, 0);
        let r = layout.InPort(id, 1);
        let q = layout.OutPort(id, 0);
        let q1 = layout.OutPort(id, 1);

        let nameStr = if name.is_empty() { "RSLatch" } else { name };
        let n1Name = format!("{}.Nand1", nameStr);
        let n2Name = format!("{}.Nand2", nameStr);
        let nand1 = NandGate::WithParent(layout, &n1Name, id);
        let nand2 = NandGate::WithParent(layout, &n2Name, id);

        layout.Connect(s, nand1.In1());
        layout.Connect(r, nand2.In1());

        layout.Connect(nand1.Out(), nand2.In2());
        layout.Connect(nand2.Out(), nand1.In2());

        layout.Connect(nand1.Out(), q);
        layout.Connect(nand2.Out(), q1);

        layout.SealModule(id);

        Self {
            _Id:    id,
            _Nand1: nand1,
            _Nand2: nand2,
            _S:     s,
            _R:     r,
            _Q:     q,
            _Q1:    q1,
        }
    }

    #[inline]
    pub const fn Id(&self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn S(&self) -> PortId
    {
        self._S
    }

    #[inline]
    pub const fn R(&self) -> PortId
    {
        self._R
    }

    #[inline]
    pub const fn Q(&self) -> PortId
    {
        self._Q
    }

    #[inline]
    pub const fn Q1(&self) -> PortId
    {
        self._Q1
    }

    #[inline]
    pub fn SetS(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._S, val);
    }

    #[inline]
    pub fn SetR(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._R, val);
    }

    #[inline]
    pub fn SetS4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._S, if val { 1 } else { 0 }, isX, isI);
    }

    #[inline]
    pub fn SetR4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._R, if val { 1 } else { 0 }, isX, isI);
    }
}

//------------------------------------------------------------------------------------------------------------------
// Clocked RS Latch.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CRSLatch
{
    pub _Id:    ModuleId,
    pub _GateS: NandGate,
    pub _GateR: NandGate,
    pub _RS:    RSLatch,
    pub _Clk1:  PortId,
    pub _Clk2:  PortId,
    pub _S:     PortId,
    pub _R:     PortId,
    pub _Q:     PortId,
    pub _Q1:    PortId,
}

impl CRSLatch
{
    #[inline]
    pub fn New(layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent(layout, name, ModuleId::None())
    }

    pub fn WithParent(layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let inDescs = [
            PortDesc::Bool("Clk1"),
            PortDesc::Bool("Clk2"),
            PortDesc::Bool("S"),
            PortDesc::Bool("R"),
        ];
        let outDescs = [PortDesc::Bool("Q"), PortDesc::Bool("Q1")];

        let id = layout.AddModule(
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::None,
        );

        let clk1 = layout.InPort(id, 0);
        let clk2 = layout.InPort(id, 1);
        let s = layout.InPort(id, 2);
        let r = layout.InPort(id, 3);
        let q = layout.OutPort(id, 0);
        let q1 = layout.OutPort(id, 1);

        let nameStr = if name.is_empty() { "CRSLatch" } else { name };
        let sName = format!("{}.GateS", nameStr);
        let rName = format!("{}.GateR", nameStr);
        let rsName = format!("{}.RS", nameStr);

        let gateS = NandGate::WithParent(layout, &sName, id);
        let gateR = NandGate::WithParent(layout, &rName, id);
        let rsLatch = RSLatch::WithParent(layout, &rsName, id);

        layout.Connect(s, gateS.In1());
        layout.Connect(clk1, gateS.In2());
        layout.Connect(clk2, gateR.In1());
        layout.Connect(r, gateR.In2());

        layout.Connect(gateS.Out(), rsLatch.S());
        layout.Connect(gateR.Out(), rsLatch.R());

        layout.Connect(rsLatch.Q(), q);
        layout.Connect(rsLatch.Q1(), q1);

        layout.SealModule(id);

        Self {
            _Id:    id,
            _GateS: gateS,
            _GateR: gateR,
            _RS:    rsLatch,
            _Clk1:  clk1,
            _Clk2:  clk2,
            _S:     s,
            _R:     r,
            _Q:     q,
            _Q1:    q1,
        }
    }

    #[inline]
    pub const fn Id(&self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn Clk1(&self) -> PortId
    {
        self._Clk1
    }

    #[inline]
    pub const fn Clk2(&self) -> PortId
    {
        self._Clk2
    }

    #[inline]
    pub const fn S(&self) -> PortId
    {
        self._S
    }

    #[inline]
    pub const fn R(&self) -> PortId
    {
        self._R
    }

    #[inline]
    pub const fn Q(&self) -> PortId
    {
        self._Q
    }

    #[inline]
    pub const fn Q1(&self) -> PortId
    {
        self._Q1
    }

    #[inline]
    pub fn SetS(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._S, val);
    }

    #[inline]
    pub fn SetR(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._R, val);
    }

    #[inline]
    pub fn SetClk(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._Clk1, val);
        engine.SetBool(self._Clk2, val);
    }

    #[inline]
    pub fn SetS4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._S, if val { 1 } else { 0 }, isX, isI);
    }

    #[inline]
    pub fn SetR4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._R, if val { 1 } else { 0 }, isX, isI);
    }

    #[inline]
    pub fn SetClk4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._Clk1, if val { 1 } else { 0 }, isX, isI);
        engine.Set(self._Clk2, if val { 1 } else { 0 }, isX, isI);
    }
}

//------------------------------------------------------------------------------------------------------------------
// Transparent D-Latch.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DLatch
{
    pub _Id:   ModuleId,
    pub _Not:  NotGate,
    pub _CRS:  CRSLatch,
    pub _D:    PortId,
    pub _DInv: PortId,
    pub _E1:   PortId,
    pub _E2:   PortId,
    pub _Q:    PortId,
    pub _Q1:   PortId,
}

impl DLatch
{
    #[inline]
    pub fn New(layout: &mut Layout, name: &str) -> Self
    {
        Self::WithParent(layout, name, ModuleId::None())
    }

    pub fn WithParent(layout: &mut Layout, name: &str, parent: ModuleId) -> Self
    {
        let inDescs = [
            PortDesc::Bool("D"),
            PortDesc::Bool("DInv"),
            PortDesc::Bool("E1"),
            PortDesc::Bool("E2"),
        ];
        let outDescs = [PortDesc::Bool("Q"), PortDesc::Bool("Q1")];

        let id = layout.AddModule(
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            KernelKind::None,
        );

        let d = layout.InPort(id, 0);
        let dInv = layout.InPort(id, 1);
        let e1 = layout.InPort(id, 2);
        let e2 = layout.InPort(id, 3);
        let q = layout.OutPort(id, 0);
        let q1 = layout.OutPort(id, 1);

        let nameStr = if name.is_empty() { "DLatch" } else { name };
        let crsName = format!("{}.CRS", nameStr);
        let invName = format!("{}.Inv", nameStr);

        let crsLatch = CRSLatch::WithParent(layout, &crsName, id);
        let notGate = NotGate::WithParent(layout, &invName, id);

        layout.Connect(d, crsLatch.S());
        layout.Connect(dInv, notGate.In());
        layout.Connect(e1, crsLatch.Clk1());
        layout.Connect(e2, crsLatch.Clk2());

        layout.Connect(notGate.Out(), crsLatch.R());

        layout.Connect(crsLatch.Q(), q);
        layout.Connect(crsLatch.Q1(), q1);

        layout.SealModule(id);

        Self {
            _Id:   id,
            _Not:  notGate,
            _CRS:  crsLatch,
            _D:    d,
            _DInv: dInv,
            _E1:   e1,
            _E2:   e2,
            _Q:    q,
            _Q1:   q1,
        }
    }

    #[inline]
    pub const fn Id(&self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn D(&self) -> PortId
    {
        self._D
    }

    #[inline]
    pub const fn DInv(&self) -> PortId
    {
        self._DInv
    }

    #[inline]
    pub const fn E1(&self) -> PortId
    {
        self._E1
    }

    #[inline]
    pub const fn E2(&self) -> PortId
    {
        self._E2
    }

    #[inline]
    pub const fn Q(&self) -> PortId
    {
        self._Q
    }

    #[inline]
    pub const fn Q1(&self) -> PortId
    {
        self._Q1
    }

    #[inline]
    pub fn SetD(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._D, val);
        engine.SetBool(self._DInv, val);
    }

    #[inline]
    pub fn SetEnable(&self, engine: &mut SimEngine, val: bool)
    {
        engine.SetBool(self._E1, val);
        engine.SetBool(self._E2, val);
    }

    #[inline]
    pub fn SetD4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._D, if val { 1 } else { 0 }, isX, isI);
        engine.Set(self._DInv, if val { 1 } else { 0 }, isX, isI);
    }

    #[inline]
    pub fn SetEnable4(&self, engine: &mut SimEngine, val: bool, isX: bool, isI: bool)
    {
        engine.Set(self._E1, if val { 1 } else { 0 }, isX, isI);
        engine.Set(self._E2, if val { 1 } else { 0 }, isX, isI);
    }
}

