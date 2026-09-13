// vm_runner.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "crew/protocol.h"
#include "rube/rube.h"
#include "silo/arr.h"

#include <cstdint>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------
// VmBus — helper for constructing zero-allocation CoroPorts transactions for VM MMIO bus.

struct VmBus
{
    static inline rube::CoroPorts Idle( void) noexcept
    {
        rube::CoroPorts p;
        p.Push( false);
        p.Push( false);
        p.Push( 0ULL);
        p.Push( 0ULL);
        return p;
    }

    static inline rube::CoroPorts Read( uint32_t addr) noexcept
    {
        rube::CoroPorts p;
        p.Push( true);
        p.Push( false);
        p.Push( static_cast< uint64_t>( addr));
        p.Push( 0ULL);
        return p;
    }

    static inline rube::CoroPorts Write( uint32_t addr, uint32_t val) noexcept
    {
        rube::CoroPorts p;
        p.Push( true);
        p.Push( true);
        p.Push( static_cast< uint64_t>( addr));
        p.Push( static_cast< uint64_t>( val));
        return p;
    }
};

//-------------------------------------------------------------------------------------------------
// VMRunner — encapsulates a virtual machine guest executing as a C++20 CoroModule.
// Communicates with its local VMAdaptor via 4 outputs (Req, Write, Addr, WData) and 2 inputs (Ack, RData).

class VMRunner
{
private:
    rube::ModuleId  _Id{};
    rube::PortId    _ReqOut{};
    rube::PortId    _WriteOut{};
    rube::PortId    _AddrOut{};
    rube::PortId    _WDataOut{};
    rube::PortId    _AckIn{};
    rube::PortId    _RDataIn{};

public:
    VMRunner( void) = default;

    VMRunner(
        rube::Layout& layout,
        const char* name,
        rube::CoroKernelFactory factory,
        rube::ModuleId parent = rube::ModuleId{})
    {
        rube::PortDesc inDescs[2] = {
            rube::PortDesc( "Ack", rube::PortType::Bool()),
            rube::PortDesc( "RData", rube::PortType::U32Val())
        };
        rube::PortDesc outDescs[4] = {
            rube::PortDesc( "Req", rube::PortType::Bool()),
            rube::PortDesc( "Write", rube::PortType::Bool()),
            rube::PortDesc( "Addr", rube::PortType::U32Val()),
            rube::PortDesc( "WData", rube::PortType::U32Val())
        };

        _Id = layout.AddCoroModule(
            name,
            parent,
            silo::Arr< const rube::PortDesc>( inDescs, 2),
            silo::Arr< const rube::PortDesc>( outDescs, 4),
            std::move( factory)
        );

        _AckIn = layout.InPort( _Id, 0);
        _RDataIn = layout.InPort( _Id, 1);
        _ReqOut = layout.OutPort( _Id, 0);
        _WriteOut = layout.OutPort( _Id, 1);
        _AddrOut = layout.OutPort( _Id, 2);
        _WDataOut = layout.OutPort( _Id, 3);
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr rube::PortId Ack( void) const noexcept { return _AckIn; }
    constexpr rube::PortId RData( void) const noexcept { return _RDataIn; }
    constexpr rube::PortId Req( void) const noexcept { return _ReqOut; }
    constexpr rube::PortId Write( void) const noexcept { return _WriteOut; }
    constexpr rube::PortId Addr( void) const noexcept { return _AddrOut; }
    constexpr rube::PortId WData( void) const noexcept { return _WDataOut; }
};

} // namespace trellis::crew
