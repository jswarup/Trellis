// fabric_node.h --------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "karst/dchan.h"
#include "karst/epu.h"
#include "karst/link.h"
#include "karst/noc.h"
#include "karst/pipe.h"
#include "rube/rube.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/fifo.h"
#include "swarm/cpu.h"

#include <cstdint>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstFabricNode — KarstHind Memory Fabric die composite model.
// Integrates the MF-NoC crossbar, 4 DDR5 memory controllers with mpipe retiming,
// 4 physical DDR5 memory channels (KarstDChan), and 4 Near-Memory EPUs (KarstVPU).

class KarstFabricNode
{
private:
    uint32_t                                                _DieId{0};
    KarstNoc                                                _Noc{};
    silo::Buff< KarstPipe>                                  _Pipes{};
    silo::Buff< std::unique_ptr< KarstDChan>>               _DChans{};
    silo::Buff< std::unique_ptr< KarstVPU>>                 _VPUs{};

public:
    KarstFabricNode( void) = default;

    KarstFabricNode(
        rube::Layout& layout,
        swarm::ComputeDevice& device,
        const char* name,
        uint32_t dieId,
        rube::ModuleId parent = rube::ModuleId{})
        : _DieId( dieId)
    {
        const std::string baseName = name ? name : ( "Hind_" + std::to_string( dieId));

        // 1. Instantiate MF-NoC crossbar
        const std::string nocName = baseName + ".Noc";
        _Noc = KarstNoc( layout, nocName.c_str(), dieId, parent);

        // 2. Instantiate 4 memory channels and 4 EPUs
        _DChans = silo::Buff< std::unique_ptr< KarstDChan>>( 4, []( uint32_t) {
            return std::unique_ptr< KarstDChan>{};
        });
        _VPUs = silo::Buff< std::unique_ptr< KarstVPU>>( 4, []( uint32_t) {
            return std::unique_ptr< KarstVPU>{};
        });
        _Pipes = silo::Buff< KarstPipe>( 4, []( uint32_t) {
            return KarstPipe{};
        });

        for ( uint32_t m = 0; m < 4; ++m) {
            const uint32_t globalChanIdx = dieId * 4 + m;
            _DChans[m] = std::make_unique< KarstDChan>( device, globalChanIdx, 4096);
            _VPUs[m]   = std::make_unique< KarstVPU>( device, globalChanIdx);

            // Instantiate mpipe retimer between NoC and MC
            const std::string pipeName = baseName + ".Pipe_" + std::to_string( m);
            _Pipes[m] = KarstPipe( layout, pipeName.c_str(), k_LinkDepth, parent);

            // Connect NoC MC request -> KarstPipe input
            layout.Connect( _Noc.McReqValid( m), _Pipes[m].InValid());
            layout.Connect( _Noc.McReqData( m),  _Pipes[m].InData());
            layout.Connect( _Pipes[m].UpReady(), _Noc.McReqReady( m));

            // Instantiate Memory Controller coroutine
            const std::string mcName = baseName + ".MC_" + std::to_string( m);
            rube::PortDesc inDescs[3] = {
                rube::PortDesc( "ReqValid",  rube::PortType::Bool()),
                rube::PortDesc( "ReqData",   rube::PortType::U64Val()),
                rube::PortDesc( "RespReady", rube::PortType::Bool())
            };
            rube::PortDesc outDescs[3] = {
                rube::PortDesc( "ReqReady",  rube::PortType::Bool()),
                rube::PortDesc( "RespValid", rube::PortType::Bool()),
                rube::PortDesc( "RespData",  rube::PortType::U64Val())
            };

            KarstDChan* dchanPtr = _DChans[m].get();

            rube::ModuleId mcModId = layout.AddCoroModule(
                mcName.c_str(),
                parent,
                silo::Arr< const rube::PortDesc>( inDescs, 3),
                silo::Arr< const rube::PortDesc>( outDescs, 3),
                [dchanPtr]() -> rube::CoroTask {
                    silo::Fifo< uint64_t, 16> respQueue;
                    bool lastRespPresented = false;

                    rube::CoroPorts in = co_await rube::CoroIn{};

                    while ( true)
                    {
                        if ( lastRespPresented && in.Get< bool>( 2) && !respQueue.IsEmpty()) {
                            respQueue.PopFront();
                        }

                        if ( in.Get< bool>( 0)) {
                            const uint64_t raw = in[1];
                            const KarstFlit flit = KarstFlit::Unpack( raw);

                            if ( flit._IsWrite) {
                                dchanPtr->WriteWord( flit._Addr, flit._Data);
                            } else {
                                const uint32_t readVal = dchanPtr->ReadWord( flit._Addr);
                                const uint64_t respRaw = KarstFlit::Pack( flit._Addr, readVal, flit._SrcId, false);
                                if ( !respQueue.IsFull()) {
                                    respQueue.PushBack( respRaw);
                                }
                            }
                        }

                        bool respValid = false;
                        uint64_t respData = 0;
                        if ( !respQueue.IsEmpty()) {
                            respValid = true;
                            respData = respQueue.Front();
                        }
                        lastRespPresented = respValid;

                        const bool reqReady = !respQueue.IsFull();

                        rube::CoroPorts out;
                        out.Push( reqReady);
                        out.Push( respValid);
                        out.Push( respData);

                        in = co_yield out;
                    }
                }
            );

            rube::PortId mcReqValidIn   = layout.InPort( mcModId, 0);
            rube::PortId mcReqDataIn    = layout.InPort( mcModId, 1);
            rube::PortId mcRespReadyIn  = layout.InPort( mcModId, 2);

            rube::PortId mcReqReadyOut  = layout.OutPort( mcModId, 0);
            rube::PortId mcRespValidOut = layout.OutPort( mcModId, 1);
            rube::PortId mcRespDataOut  = layout.OutPort( mcModId, 2);

            // Connect KarstPipe output -> MC input
            layout.Connect( _Pipes[m].OutValid(), mcReqValidIn);
            layout.Connect( _Pipes[m].OutData(),  mcReqDataIn);
            layout.Connect( mcReqReadyOut,        _Pipes[m].DownReady());

            // Connect MC response -> NoC response input
            layout.Connect( mcRespValidOut,       _Noc.McRespValid( m));
            layout.Connect( mcRespDataOut,        _Noc.McRespData( m));
            layout.Connect( _Noc.McRespReady( m), mcRespReadyIn);
        }
    }

    constexpr uint32_t DieId( void) const noexcept { return _DieId; }

    KarstLink TwLink( uint32_t port) const noexcept
    {
        return _Noc.TwLink( port);
    }

    KarstDChan& DChan( uint32_t mcIdx) noexcept
    {
        return *_DChans[mcIdx];
    }

    const KarstDChan& DChan( uint32_t mcIdx) const noexcept
    {
        return *_DChans[mcIdx];
    }

    KarstVPU& VPU( uint32_t mcIdx) noexcept
    {
        return *_VPUs[mcIdx];
    }

    const KarstVPU& VPU( uint32_t mcIdx) const noexcept
    {
        return *_VPUs[mcIdx];
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

