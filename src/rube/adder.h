// adder.h --------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/engine.h"
#include "rube/gates.h"
#include "rube/layout.h"
#include "rube/module.h"
#include "rube/port.h"
#include "rube/reg.h"
#include "silo/buff.h"
#include "silo/stash.h"

#include <cstdint>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// 1-Bit Half Adder (XOR sum + AND carry).
// Modeled directly from Kosh rube/adder.rs.

class HalfAdder
{
    ModuleId    _Id{};
    XorGate     _Xor{};
    AndGate     _And{};
    PortId      _In1{};
    PortId      _In2{};
    PortId      _Sum{};
    PortId      _Carry{};

public:
    constexpr HalfAdder( void) noexcept = default;

    HalfAdder( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[2] = {PortDesc::Bool( "a"), PortDesc::Bool( "b")};
        PortDesc outDescs[2] = {PortDesc::Bool( "sum"), PortDesc::Bool( "carry")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 2),
            silo::Arr< const PortDesc>( outDescs, 2),
            KernelKind::None()
        );

        _In1   = layout.InPort( _Id, 0);
        _In2   = layout.InPort( _Id, 1);
        _Sum   = layout.OutPort( _Id, 0);
        _Carry = layout.OutPort( _Id, 1);

        const std::string nameStr = name ? name : "HalfAdder";
        _Xor = XorGate( layout, ( nameStr + ".Xor").c_str(), _Id);
        _And = AndGate( layout, ( nameStr + ".And").c_str(), _Id);

        layout.Connect( _In1, _Xor.In1());
        layout.Connect( _In2, _Xor.In2());
        layout.Connect( _In1, _And.In1());
        layout.Connect( _In2, _And.In2());

        layout.Connect( _Xor.Out(), _Sum);
        layout.Connect( _And.Out(), _Carry);

        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId In1( void) const noexcept { return _In1; }
    constexpr PortId In2( void) const noexcept { return _In2; }
    constexpr PortId Sum( void) const noexcept { return _Sum; }
    constexpr PortId Carry( void) const noexcept { return _Carry; }

    void SetA( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _In1, val);
    }

    void SetB( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _In2, val);
    }
};

//-------------------------------------------------------------------------------------------------
// 1-Bit Full Adder (2 HalfAdders + OR gate).

class FullAdder
{
    ModuleId    _Id{};
    HalfAdder   _HA1{};
    HalfAdder   _HA2{};
    OrGate      _Or{};
    PortId      _In1{};
    PortId      _In2{};
    PortId      _CIn{};
    PortId      _Sum{};
    PortId      _Carry{};

public:
    constexpr FullAdder( void) noexcept = default;

    FullAdder( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        PortDesc inDescs[3] = {PortDesc::Bool( "a"), PortDesc::Bool( "b"), PortDesc::Bool( "cin")};
        PortDesc outDescs[2] = {PortDesc::Bool( "sum"), PortDesc::Bool( "carry")};

        _Id = layout.AddModule(
            name,
            parent,
            silo::Arr< const PortDesc>( inDescs, 3),
            silo::Arr< const PortDesc>( outDescs, 2),
            KernelKind::None()
        );

        _In1   = layout.InPort( _Id, 0);
        _In2   = layout.InPort( _Id, 1);
        _CIn   = layout.InPort( _Id, 2);
        _Sum   = layout.OutPort( _Id, 0);
        _Carry = layout.OutPort( _Id, 1);

        const std::string nameStr = name ? name : "FullAdder";
        _HA1 = HalfAdder( layout, ( nameStr + ".HA1").c_str(), _Id);
        _HA2 = HalfAdder( layout, ( nameStr + ".HA2").c_str(), _Id);
        _Or  = OrGate( layout, ( nameStr + ".Or").c_str(), _Id);

        // Pass-down
        layout.Connect( _In1, _HA1.In1());
        layout.Connect( _In2, _HA1.In2());
        layout.Connect( _CIn, _HA2.In2());

        // Sibling-to-sibling
        layout.Connect( _HA1.Sum(), _HA2.In1());
        layout.Connect( _HA1.Carry(), _Or.In1());
        layout.Connect( _HA2.Carry(), _Or.In2());

        // Pass-up
        layout.Connect( _HA2.Sum(), _Sum);
        layout.Connect( _Or.Out(), _Carry);

        layout.SealModule( _Id);
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId In1( void) const noexcept { return _In1; }
    constexpr PortId In2( void) const noexcept { return _In2; }
    constexpr PortId CIn( void) const noexcept { return _CIn; }
    constexpr PortId Sum( void) const noexcept { return _Sum; }
    constexpr PortId Carry( void) const noexcept { return _Carry; }

    void SetA( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _In1, val);
    }

    void SetB( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _In2, val);
    }

    void SetCIn( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _CIn, val);
    }
};

//-------------------------------------------------------------------------------------------------
// N-Bit Ripple Carry Adder.

template < uint32_t N>
class Adder
{
    ModuleId                _Id{};
    silo::Buff< FullAdder>  _Bits{};
    silo::Buff< PortId>     _A{};
    silo::Buff< PortId>     _B{};
    PortId                  _CIn{};
    silo::Buff< PortId>     _Sum{};
    PortId                  _Carry{};

public:
    constexpr Adder( void) noexcept = default;

    Adder( Layout& layout, const char* name, ModuleId parent = ModuleId{})
    {
        silo::Stash< PortDesc> inDescs;
        for ( uint32_t i = 0; i < N; ++i) {
            inDescs.PushBack( PortDesc::Bool( "a" + std::to_string( i)));
        }
        for ( uint32_t i = 0; i < N; ++i) {
            inDescs.PushBack( PortDesc::Bool( "b" + std::to_string( i)));
        }
        inDescs.PushBack( PortDesc::Bool( "cin"));

        silo::Stash< PortDesc> outDescs;
        for ( uint32_t i = 0; i < N; ++i) {
            outDescs.PushBack( PortDesc::Bool( "sum" + std::to_string( i)));
        }
        outDescs.PushBack( PortDesc::Bool( "carry"));

        _Id = layout.AddModule(
            name,
            parent,
            inDescs.AsArr(),
            outDescs.AsArr(),
            KernelKind::None()
        );

        silo::Stash< PortId> aPorts;
        silo::Stash< PortId> bPorts;
        for ( uint32_t i = 0; i < N; ++i) {
            aPorts.PushBack( layout.InPort( _Id, i));
            bPorts.PushBack( layout.InPort( _Id, i + N));
        }
        _CIn = layout.InPort( _Id, 2 * N);

        silo::Stash< PortId> sumPorts;
        for ( uint32_t i = 0; i < N; ++i) {
            sumPorts.PushBack( layout.OutPort( _Id, i));
        }
        _Carry = layout.OutPort( _Id, N);

        silo::Stash< FullAdder> fullAdders;
        const std::string nameStr = name ? name : "Adder";

        for ( uint32_t i = 0; i < N; ++i) {
            const std::string bitName = nameStr + ".Bit" + std::to_string( i);
            FullAdder bit( layout, bitName.c_str(), _Id);

            layout.Connect( aPorts[i], bit.In1());
            layout.Connect( bPorts[i], bit.In2());

            if ( i == 0) {
                layout.Connect( _CIn, bit.CIn());
            } else {
                layout.Connect( fullAdders[i - 1].Carry(), bit.CIn());
            }

            layout.Connect( bit.Sum(), sumPorts[i]);
            fullAdders.PushBack( bit);
        }

        layout.Connect( fullAdders[N - 1].Carry(), _Carry);
        layout.SealModule( _Id);

        _Bits = fullAdders.ExtractBuff();
        _A = aPorts.ExtractBuff();
        _B = bPorts.ExtractBuff();
        _Sum = sumPorts.ExtractBuff();
    }

    constexpr ModuleId Id( void) const noexcept { return _Id; }
    constexpr PortId Carry( void) const noexcept { return _Carry; }

    void SetA( SimEngine& engine, uint32_t val) const
    {
        for ( uint32_t i = 0; i < N; ++i) {
            const bool bit = ( ( val >> i) & 1) != 0;
            engine.SetPortBool( _A[i], Reg::FromBool( bit));
        }
    }

    void SetB( SimEngine& engine, uint32_t val) const
    {
        for ( uint32_t i = 0; i < N; ++i) {
            const bool bit = ( ( val >> i) & 1) != 0;
            engine.SetPortBool( _B[i], Reg::FromBool( bit));
        }
    }

    void SetCarryIn( SimEngine& engine, Reg val) const
    {
        engine.SetPortBool( _CIn, val);
    }

    uint32_t GetSum( const SimEngine& engine) const
    {
        uint32_t sum = 0;
        for ( uint32_t i = 0; i < N; ++i) {
            const Reg bit = engine.GetPortBool( _Sum[i]);
            if ( bit.IsTrue()) {
                sum |= ( 1u << i);
            }
        }
        return sum;
    }
};

} // namespace trellis::rube
