// latches.h ------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/engine.h"
#include "rube/gates.h"
#include "rube/layout.h"
#include "rube/module.h"
#include "rube/port.h"
#include "rube/reg.h"

#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// Asynchronous RS Latch (Cross-coupled NAND gates).
// Modeled directly from Kosh rube/latches.rs.

class RSLatch
{
    ModuleId    _Id{};
    NandGate    _Nand1{};
    NandGate    _Nand2{};
    PortId      _S{};
    PortId      _R{};
    PortId      _Q{};
    PortId      _Q1{};

public:
    constexpr RSLatch( void) noexcept = default;

    RSLatch( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[2] = {PortDesc::Bool( "S"), PortDesc::Bool( "R")};
        PortDesc outDescs[2] = {PortDesc::Bool( "Q"), PortDesc::Bool( "Q1")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 2),
            silo::Arr< const PortDesc>( outDescs, 2),
            KernelKind::None()
        );

        _S  = layout.InPort( _Id, 0);
        _R  = layout.InPort( _Id, 1);
        _Q  = layout.OutPort( _Id, 0);
        _Q1 = layout.OutPort( _Id, 1);

        const std::string nameStr = name ? name : "RSLatch";
        _Nand1 = NandGate( layout, ( nameStr + ".Nand1").c_str(), _Id);
        _Nand2 = NandGate( layout, ( nameStr + ".Nand2").c_str(), _Id);

        layout.Connect( _S, _Nand1.In1());
        layout.Connect( _R, _Nand2.In1());

        layout.Connect( _Nand1.Out(), _Nand2.In2());
        layout.Connect( _Nand2.Out(), _Nand1.In2());

        layout.Connect( _Nand1.Out(), _Q);
        layout.Connect( _Nand2.Out(), _Q1);

        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId S( void) const noexcept { return _S; }
    constexpr PortId R( void) const noexcept { return _R; }
    constexpr PortId Q( void) const noexcept { return _Q; }
    constexpr PortId Q1( void) const noexcept { return _Q1; }

    void SetS( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _S, val);
    }

    void SetR( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _R, val);
    }
};

//-------------------------------------------------------------------------------------------------
// Clocked RS Latch.

class CRSLatch
{
    ModuleId    _Id{};
    NandGate    _GateS{};
    NandGate    _GateR{};
    RSLatch     _RS{};
    PortId      _Clk1{};
    PortId      _Clk2{};
    PortId      _S{};
    PortId      _R{};
    PortId      _Q{};
    PortId      _Q1{};

public:
    constexpr CRSLatch( void) noexcept = default;

    CRSLatch( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[4] = {
            PortDesc::Bool( "Clk1"),
            PortDesc::Bool( "Clk2"),
            PortDesc::Bool( "S"),
            PortDesc::Bool( "R")
        };
        PortDesc outDescs[2] = {PortDesc::Bool( "Q"), PortDesc::Bool( "Q1")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 4),
            silo::Arr< const PortDesc>( outDescs, 2),
            KernelKind::None()
        );

        _Clk1 = layout.InPort( _Id, 0);
        _Clk2 = layout.InPort( _Id, 1);
        _S    = layout.InPort( _Id, 2);
        _R    = layout.InPort( _Id, 3);
        _Q    = layout.OutPort( _Id, 0);
        _Q1   = layout.OutPort( _Id, 1);

        const std::string nameStr = name ? name : "CRSLatch";
        _GateS = NandGate( layout, ( nameStr + ".GateS").c_str(), _Id);
        _GateR = NandGate( layout, ( nameStr + ".GateR").c_str(), _Id);
        _RS    = RSLatch( layout, ( nameStr + ".RS").c_str(), _Id);

        layout.Connect( _S, _GateS.In1());
        layout.Connect( _Clk1, _GateS.In2());
        layout.Connect( _Clk2, _GateR.In1());
        layout.Connect( _R, _GateR.In2());

        layout.Connect( _GateS.Out(), _RS.S());
        layout.Connect( _GateR.Out(), _RS.R());

        layout.Connect( _RS.Q(), _Q);
        layout.Connect( _RS.Q1(), _Q1);

        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId Clk1( void) const noexcept { return _Clk1; }
    constexpr PortId Clk2( void) const noexcept { return _Clk2; }
    constexpr PortId S( void) const noexcept { return _S; }
    constexpr PortId R( void) const noexcept { return _R; }
    constexpr PortId Q( void) const noexcept { return _Q; }
    constexpr PortId Q1( void) const noexcept { return _Q1; }

    void SetS( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _S, val);
    }

    void SetR( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _R, val);
    }

    void SetClk( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _Clk1, val);
        engine.SetPortBool( _Clk2, val);
    }
};

//-------------------------------------------------------------------------------------------------
// Transparent D-Latch.

class DLatch
{
    ModuleId    _Id{};
    NotGate     _Not{};
    CRSLatch    _CRS{};
    PortId      _D{};
    PortId      _DInv{};
    PortId      _E1{};
    PortId      _E2{};
    PortId      _Q{};
    PortId      _Q1{};

public:
    constexpr DLatch( void) noexcept = default;

    DLatch( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[4] = {
            PortDesc::Bool( "D"),
            PortDesc::Bool( "DInv"),
            PortDesc::Bool( "E1"),
            PortDesc::Bool( "E2")
        };
        PortDesc outDescs[2] = {PortDesc::Bool( "Q"), PortDesc::Bool( "Q1")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 4),
            silo::Arr< const PortDesc>( outDescs, 2),
            KernelKind::None()
        );

        _D    = layout.InPort( _Id, 0);
        _DInv = layout.InPort( _Id, 1);
        _E1   = layout.InPort( _Id, 2);
        _E2   = layout.InPort( _Id, 3);
        _Q    = layout.OutPort( _Id, 0);
        _Q1   = layout.OutPort( _Id, 1);

        const std::string nameStr = name ? name : "DLatch";
        _CRS = CRSLatch( layout, ( nameStr + ".CRS").c_str(), _Id);
        _Not = NotGate( layout, ( nameStr + ".Inv").c_str(), _Id);

        layout.Connect( _D, _CRS.S());
        layout.Connect( _DInv, _Not.In());
        layout.Connect( _E1, _CRS.Clk1());
        layout.Connect( _E2, _CRS.Clk2());

        layout.Connect( _Not.Out(), _CRS.R());

        layout.Connect( _CRS.Q(), _Q);
        layout.Connect( _CRS.Q1(), _Q1);

        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId D( void) const noexcept { return _D; }
    constexpr PortId DInv( void) const noexcept { return _DInv; }
    constexpr PortId E1( void) const noexcept { return _E1; }
    constexpr PortId E2( void) const noexcept { return _E2; }
    constexpr PortId Q( void) const noexcept { return _Q; }
    constexpr PortId Q1( void) const noexcept { return _Q1; }

    void SetD( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _D, val);
        engine.SetPortBool( _DInv, val);
    }

    void SetEnable( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _E1, val);
        engine.SetPortBool( _E2, val);
    }
};

} // namespace trellis::rube
