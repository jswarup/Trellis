// netlist.h ------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/port.h"
#include "rube/trigger.h"
#include "silo/buff.h"
#include "silo/dset.h"
#include "silo/stash.h"

#include <cassert>
#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// Netlist — manages port connectivity and trigger ID mapping via union-find DisjointSet.
// Modeled directly from Kosh rube/netlist.rs.

class Netlist
{
public:
    silo::DisjointSet       _Equiv{};
    silo::Stash< PortId>    _Driver{};
    silo::Stash< TriggerId> _RootTrigger{};
    uint32_t                _NextTriggerId{0};
    silo::Stash< PortType>  _TriggerTypes{};

public:
    constexpr Netlist( void) noexcept = default;

    void Grow( uint32_t count)
    {
        _Equiv.Grow( count);
        for ( uint32_t i = 0; i < count; ++i) {
            _Driver.PushBack( PortId{0xFFFF'FFFF});
            _RootTrigger.PushBack( 0xFFFF'FFFF);
        }
    }

    uint32_t FindRootConst( PortId port) const
    {
        return _Equiv.FindConst( port.Index());
    }

    uint32_t FindRoot( PortId port)
    {
        return _Equiv.Find( port.Index());
    }

    bool Connect( PortId driver, PortId sink)
    {
        const uint32_t  sinkIdx = sink.Index();
        const PortId    existingDriver = _Driver[sinkIdx];
        if ( existingDriver._Id != 0xFFFF'FFFF) {
            if ( existingDriver != driver) {
                // Duplicate driver conflict
                return false;
            }
        }

        _Driver[sinkIdx] = driver;
        _Equiv.Union( driver.Index(), sink.Index());
        return true;
    }

    PortId DriverOf( PortId port)
    {
        const uint32_t root = FindRoot( port);
        return _Driver[root];
    }

    TriggerId AssignTrigger( uint32_t rootIdx, PortType portType)
    {
        const uint32_t  actualRoot = _Equiv.Find( rootIdx);
        const TriggerId existing = _RootTrigger[actualRoot];
        if ( existing != 0xFFFF'FFFF) {
            return existing;
        }

        const TriggerId trigId = _NextTriggerId++;
        _RootTrigger[actualRoot] = trigId;
        _TriggerTypes.PushBack( portType);
        return trigId;
    }

    TriggerId TriggerOf( PortId port)
    {
        const uint32_t root = FindRoot( port);
        return _RootTrigger[root];
    }

    bool HasTrigger( PortId port)
    {
        const uint32_t root = FindRoot( port);
        return _RootTrigger[root] != 0xFFFF'FFFF;
    }

    bool HasTriggerConst( PortId port) const
    {
        const uint32_t root = FindRootConst( port);
        return _RootTrigger[root] != 0xFFFF'FFFF;
    }

    silo::Buff< TriggerId> BuildPortToTrigger( void)
    {
        const uint32_t          count = _Equiv.Size();
        silo::Buff< TriggerId>  portToTrigger( count, [&]( uint32_t i) {
            const uint32_t      root = _Equiv.Find( i);
            const TriggerId     trig = _RootTrigger[root];
            assert( trig != 0xFFFF'FFFF && "Port index was not assigned a TriggerId before build");
            return trig;
        });
        return portToTrigger;
    }

    silo::Buff< TriggerId> BuildPortToTriggerConst( void) const
    {
        const uint32_t          count = _Equiv.Size();
        silo::Buff< TriggerId>  portToTrigger( count, [&]( uint32_t i) {
            const uint32_t      root = _Equiv.FindConst( i);
            const TriggerId     trig = _RootTrigger[root];
            assert( trig != 0xFFFF'FFFF && "Port index was not assigned a TriggerId before build");
            return trig;
        });
        return portToTrigger;
    }

    uint32_t TriggerCount( void) const noexcept
    {
        return _NextTriggerId;
    }

    PortType TriggerType( TriggerId trigId) const noexcept
    {
        return _TriggerTypes[trigId];
    }
};

} // namespace trellis::rube
