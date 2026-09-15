// fabric.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "karst/dchan.h"
#include "karst/epu.h"
#include "karst/fabric_node.h"
#include "karst/host_node.h"
#include "karst/link.h"
#include "rube/rube.h"
#include "silo/buff.h"
#include "swarm/cpu.h"

#include <cstdint>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// Aggregate fabric metrics across all host ports, memory channels, and EPUs.

struct KarstStats
{
    uint32_t            _TotalTxCount{0};
    uint32_t            _TotalRxCount{0};
    uint64_t            _TotalBytesWritten{0};
    uint64_t            _TotalBytesRead{0};
    uint32_t            _TotalVPUDispatches{0};
    size_t              _CycleCount{0};
};

//-------------------------------------------------------------------------------------------------
// KarstFabric — top-level system framework orchestrating the balanced Karst(8,8) topology.
// Wires 8 KarstFore host nodes with 2 KarstHind Memory Fabric dies over KarstLinks,
// manages Rube SimEngine simulation, and provides Swarm compute integration.

class KarstFabric
{
private:
    swarm::ComputeDevice                    _ComputeDevice{swarm::BackendKind::Cpu};
    rube::Layout                            _Layout{};
    rube::SimEngine                         _Engine{};

    silo::Buff< std::unique_ptr< KarstHostNode>>   _Hosts{};
    silo::Buff< std::unique_ptr< KarstFabricNode>> _Fabrics{};

public:
    KarstFabric( void)
    {
        BuildFabric();
    }

    explicit KarstFabric( uint32_t workers)
        : _ComputeDevice( swarm::BackendKind::Cpu, workers)
    {
        BuildFabric();
        if ( workers > 1) {
            _Engine.WithMode( rube::SimEngineMode::Parallel( workers));
        }
    }

    void SetEngineMode( rube::SimEngineMode mode) noexcept
    {
        _Engine.WithMode( mode);
    }

    rube::SimEngine& Engine( void) noexcept
    {
        return _Engine;
    }

    const rube::SimEngine& Engine( void) const noexcept
    {
        return _Engine;
    }

    KarstHostNode& Host( uint32_t hostId)
    {
        return *_Hosts[hostId];
    }

    const KarstHostNode& Host( uint32_t hostId) const
    {
        return *_Hosts[hostId];
    }

    KarstFabricNode& FabricNode( uint32_t dieId)
    {
        return *_Fabrics[dieId];
    }

    const KarstFabricNode& FabricNode( uint32_t dieId) const
    {
        return *_Fabrics[dieId];
    }

    KarstDChan& DChan( uint32_t globalChanIdx)
    {
        const uint32_t dieId = globalChanIdx / 4U;
        const uint32_t mcIdx = globalChanIdx % 4U;
        return _Fabrics[dieId]->DChan( mcIdx);
    }

    const KarstDChan& DChan( uint32_t globalChanIdx) const
    {
        const uint32_t dieId = globalChanIdx / 4U;
        const uint32_t mcIdx = globalChanIdx % 4U;
        return _Fabrics[dieId]->DChan( mcIdx);
    }

    KarstVPU& VPU( uint32_t globalVPUIdx)
    {
        const uint32_t dieId = globalVPUIdx / 4U;
        const uint32_t mcIdx = globalVPUIdx % 4U;
        return _Fabrics[dieId]->VPU( mcIdx);
    }

    const KarstVPU& VPU( uint32_t globalVPUIdx) const
    {
        const uint32_t dieId = globalVPUIdx / 4U;
        const uint32_t mcIdx = globalVPUIdx % 4U;
        return _Fabrics[dieId]->VPU( mcIdx);
    }

    void PostHostWrite( uint32_t hostId, uint32_t addr, uint32_t data)
    {
        if ( hostId < k_HostsPerFabric) {
            _Hosts[hostId]->PostWrite( addr, data);
        }
    }

    void PostHostRead( uint32_t hostId, uint32_t addr)
    {
        if ( hostId < k_HostsPerFabric) {
            _Hosts[hostId]->PostRead( addr);
        }
    }

    bool PopHostResponse( uint32_t hostId, HostResponse& out)
    {
        if ( hostId < k_HostsPerFabric) {
            return _Hosts[hostId]->PopResponse( out);
        }
        return false;
    }

    uint32_t Advance( uint32_t ticks = 1)
    {
        for ( uint32_t i = 0; i < ticks; ++i) {
            _Engine.Drive();
        }
        return static_cast< uint32_t>( _Engine._CycleCount);
    }

    KarstStats Stats( void) const
    {
        KarstStats total{};
        total._CycleCount = _Engine._CycleCount;

        for ( uint32_t h = 0; h < k_HostsPerFabric; ++h) {
            const HostStats hs = _Hosts[h]->Stats();
            total._TotalTxCount += hs._TxCount;
            total._TotalRxCount += hs._RxCount;
        }

        for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
            const DChanStats ds = DChan( c).Stats();
            total._TotalBytesWritten += ds._BytesWritten;
            total._TotalBytesRead    += ds._BytesRead;
        }

        for ( uint32_t e = 0; e < k_DChansPerFabric; ++e) {
            total._TotalVPUDispatches += VPU( e).Dispatches();
        }

        return total;
    }

private:
    void BuildFabric( void)
    {
        // 1. Instantiate 8 KarstFore host nodes
        _Hosts = silo::Buff< std::unique_ptr< KarstHostNode>>( k_HostsPerFabric, []( uint32_t) {
            return std::unique_ptr< KarstHostNode>{};
        });
        for ( uint32_t h = 0; h < k_HostsPerFabric; ++h) {
            const std::string name = "Fore_" + std::to_string( h);
            _Hosts[h] = std::make_unique< KarstHostNode>( _Layout, name.c_str(), h);
        }

        // 2. Instantiate 2 KarstHind Memory Fabric nodes (each with 4 DChans and 4 EPUs)
        _Fabrics = silo::Buff< std::unique_ptr< KarstFabricNode>>( k_HindDiesPerFabric, []( uint32_t) {
            return std::unique_ptr< KarstFabricNode>{};
        });
        for ( uint32_t d = 0; d < k_HindDiesPerFabric; ++d) {
            const std::string name = "Hind_" + std::to_string( d);
            _Fabrics[d] = std::make_unique< KarstFabricNode>( _Layout, _ComputeDevice, name.c_str(), d);
        }

        // 3. Wire KarstLink links:
        // Die 0 (KarstHind #0):
        //   Ports 0..3  <-> Hosts 0..3 Link0 (Primary)
        //   Ports 4..7  <-> Hosts 4..7 Link1 (Cross-home)
        //   Ports 8..9  <-> Die 1 Ports 8..9 (Inter-die Hind2 link)
        for ( uint32_t h = 0; h < 4; ++h) {
            KarstLink::Connect( _Layout, _Fabrics[0]->KlLink( h), _Hosts[h]->Link0());
        }
        for ( uint32_t h = 0; h < 4; ++h) {
            KarstLink::Connect( _Layout, _Fabrics[0]->KlLink( 4 + h), _Hosts[4 + h]->Link1());
        }

        // Die 1 (KarstHind #1):
        //   Ports 0..3  <-> Hosts 4..7 Link0 (Primary)
        //   Ports 4..7  <-> Hosts 0..3 Link1 (Cross-home)
        //   Ports 8..9  <-> Die 0 Ports 8..9 (Inter-die Hind link)
        for ( uint32_t h = 0; h < 4; ++h) {
            KarstLink::Connect( _Layout, _Fabrics[1]->KlLink( h), _Hosts[4 + h]->Link0());
        }
        for ( uint32_t h = 0; h < 4; ++h) {
            KarstLink::Connect( _Layout, _Fabrics[1]->KlLink( 4 + h), _Hosts[h]->Link1());
        }

        // Inter-die links between KarstHind #0 and KarstHind #1
        KarstLink::Connect( _Layout, _Fabrics[0]->KlLink( 8), _Fabrics[1]->KlLink( 8));
        KarstLink::Connect( _Layout, _Fabrics[0]->KlLink( 9), _Fabrics[1]->KlLink( 9));

        // 4. Freeze layout and compile simulation engine
        _Layout.Freeze();
        _Engine = rube::SimEngine::Create( _Layout);
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

