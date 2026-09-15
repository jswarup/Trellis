// noc.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "karst/link.h"
#include "rube/rube.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/fifo.h"

#include <cstdint>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstNoc — KarstHind MF-NoC crossbar switch coroutine.
// KarstNoc — KarstHind MemFabric crossbar switch coroutine.
// Switches 10 KarstLink ports (KL0..KL9) and 4 local DDR5 Memory Controllers (MC0..MC3).
// Implements address decoding, 1 kB striped memory interleaving, and inter-die KarstLink routing.

class KarstNoc
{
private:
    rube::ModuleId      _Id{};
    uint32_t            _DieId{0};

    // Cached port IDs for fast accessor lookups
    silo::Buff< rube::PortId> _KlRxValidIn{};
    silo::Buff< rube::PortId> _KlRxDataIn{};
    silo::Buff< rube::PortId> _KlTxReadyIn{};
    silo::Buff< rube::PortId> _McReqReadyIn{};
    silo::Buff< rube::PortId> _McRespValidIn{};
    silo::Buff< rube::PortId> _McRespDataIn{};

    silo::Buff< rube::PortId> _KlRxReadyOut{};
    silo::Buff< rube::PortId> _KlTxValidOut{};
    silo::Buff< rube::PortId> _KlTxDataOut{};
    silo::Buff< rube::PortId> _McReqValidOut{};
    silo::Buff< rube::PortId> _McReqDataOut{};
    silo::Buff< rube::PortId> _McRespReadyOut{};

public:
    KarstNoc( void) = default;

    KarstNoc(
        rube::Layout& layout,
        const char* name,
        uint32_t dieId,
        rube::ModuleId parent = rube::ModuleId{})
        : _DieId( dieId)
    {
        // 42 Input ports:
        //   [0..29]:  10 KL ports x (KlRxValid, KlRxData, KlTxReady)
        //   [30..41]: 4 MC ports  x (McReqReady, McRespValid, McRespData)
        silo::Buff< rube::PortDesc> inDescs( 42, []( uint32_t) {
            return rube::PortDesc{};
        });
        for ( uint32_t t = 0; t < 10; ++t) {
            const std::string pfx = "Kl" + std::to_string( t);
            inDescs[3 * t + 0] = rube::PortDesc( ( pfx + "_RxValid").c_str(), rube::PortType::Bool());
            inDescs[3 * t + 1] = rube::PortDesc( ( pfx + "_RxData").c_str(),  rube::PortType::U64Val());
            inDescs[3 * t + 2] = rube::PortDesc( ( pfx + "_TxReady").c_str(), rube::PortType::Bool());
        }
        for ( uint32_t m = 0; m < 4; ++m) {
            const std::string pfx = "Mc" + std::to_string( m);
            inDescs[30 + 3 * m + 0] = rube::PortDesc( ( pfx + "_ReqReady").c_str(),  rube::PortType::Bool());
            inDescs[30 + 3 * m + 1] = rube::PortDesc( ( pfx + "_RespValid").c_str(), rube::PortType::Bool());
            inDescs[30 + 3 * m + 2] = rube::PortDesc( ( pfx + "_RespData").c_str(),  rube::PortType::U64Val());
        }

        // 42 Output ports:
        //   [0..29]:  10 KL ports x (KlRxReady, KlTxValid, KlTxData)
        //   [30..41]: 4 MC ports  x (McReqValid, McReqData, McRespReady)
        silo::Buff< rube::PortDesc> outDescs( 42, []( uint32_t) {
            return rube::PortDesc{};
        });
        for ( uint32_t t = 0; t < 10; ++t) {
            const std::string pfx = "Kl" + std::to_string( t);
            outDescs[3 * t + 0] = rube::PortDesc( ( pfx + "_RxReady").c_str(), rube::PortType::Bool());
            outDescs[3 * t + 1] = rube::PortDesc( ( pfx + "_TxValid").c_str(), rube::PortType::Bool());
            outDescs[3 * t + 2] = rube::PortDesc( ( pfx + "_TxData").c_str(),  rube::PortType::U64Val());
        }
        for ( uint32_t m = 0; m < 4; ++m) {
            const std::string pfx = "Mc" + std::to_string( m);
            outDescs[30 + 3 * m + 0] = rube::PortDesc( ( pfx + "_ReqValid").c_str(),  rube::PortType::Bool());
            outDescs[30 + 3 * m + 1] = rube::PortDesc( ( pfx + "_ReqData").c_str(),   rube::PortType::U64Val());
            outDescs[30 + 3 * m + 2] = rube::PortDesc( ( pfx + "_RespReady").c_str(), rube::PortType::Bool());
        }

        _Id = layout.AddCoroModule(
            name,
            parent,
            inDescs.AsArr(),
            outDescs.AsArr(),
            [dieId]() -> rube::CoroTask {
                silo::Fifo< uint64_t, 16> mcReqQueue[4];
                silo::Fifo< uint64_t, 16> klTxQueue[10];
                silo::Fifo< uint64_t, 16> klRxQueue[10];
                silo::Fifo< uint64_t, 16> mcRespQueue[4];

                bool lastMcReqPresented[4] = {false, false, false, false};
                bool lastKlTxPresented[10] = {};

                rube::CoroPorts in = co_await rube::CoroIn{};

                while ( true)
                {
                    // 1. Retire successfully transferred outputs
                    for ( uint32_t m = 0; m < 4; ++m) {
                        const bool reqReady = in.Get< bool>( 30 + 3 * m + 0);
                        if ( lastMcReqPresented[m] && reqReady && !mcReqQueue[m].IsEmpty()) {
                            mcReqQueue[m].PopFront();
                        }
                    }
                    for ( uint32_t t = 0; t < 10; ++t) {
                        const bool txReady = in.Get< bool>( 3 * t + 2);
                        if ( lastKlTxPresented[t] && txReady && !klTxQueue[t].IsEmpty()) {
                            klTxQueue[t].PopFront();
                        }
                    }

                    // 2. Capture ingress independently before routing it to shared queues.
                    for ( uint32_t t = 0; t < 10; ++t) {
                        const bool rxValid = in.Get< bool>( 3 * t + 0);
                        if ( rxValid && !klRxQueue[t].IsFull()) {
                            klRxQueue[t].PushBack( in[3 * t + 1]);
                        }
                    }

                    // 3. Capture MC responses independently before routing them to shared links.
                    for ( uint32_t m = 0; m < 4; ++m) {
                        const bool respValid = in.Get< bool>( 30 + 3 * m + 1);
                        if ( respValid && !mcRespQueue[m].IsFull()) {
                            mcRespQueue[m].PushBack( in[30 + 3 * m + 2]);
                        }
                    }

                    // 4. Route buffered ingress to its selected output queue.
                    for ( uint32_t t = 0; t < 10; ++t) {
                        if ( !klRxQueue[t].IsEmpty()) {
                            const uint64_t raw = klRxQueue[t].Front();
                            const KarstFlit flit = KarstFlit::Unpack( raw);

                            const uint32_t targetDie = ( flit._Addr >> 12) & 1U;
                            if ( targetDie == dieId) {
                                const uint32_t mcIdx = ( flit._Addr >> 10) & 3U;
                                if ( !mcReqQueue[mcIdx].IsFull()) {
                                    mcReqQueue[mcIdx].PushBack( raw);
                                    klRxQueue[t].PopFront();
                                }
                            } else {
                                if ( !klTxQueue[8].IsFull()) {
                                    klTxQueue[8].PushBack( raw);
                                    klRxQueue[t].PopFront();
                                }
                            }
                        }
                    }

                    for ( uint32_t m = 0; m < 4; ++m) {
                        if ( !mcRespQueue[m].IsEmpty()) {
                            const uint64_t raw = mcRespQueue[m].Front();
                            const KarstFlit flit = KarstFlit::Unpack( raw);
                            const uint32_t targetKl = ( dieId == 0)
                                ? flit._SrcId % 8U
                                : ( ( flit._SrcId >= 4) ? ( flit._SrcId - 4) : ( flit._SrcId + 4)) % 8U;
                            if ( !klTxQueue[targetKl].IsFull()) {
                                klTxQueue[targetKl].PushBack( raw);
                                mcRespQueue[m].PopFront();
                            }
                        }
                    }

                    // 5. Drive outputs
                    rube::CoroPorts out;

                    // 5a. 10 KL ports
                    for ( uint32_t t = 0; t < 10; ++t) {
                        bool txValid = false;
                        uint64_t txData = 0;
                        if ( !klTxQueue[t].IsEmpty()) {
                            txValid = true;
                            txData = klTxQueue[t].Front();
                        }
                        lastKlTxPresented[t] = txValid;

                        out.Push( !klRxQueue[t].IsFull());
                        out.Push( txValid);
                        out.Push( txData);
                    }

                    // 5b. 4 MC ports
                    for ( uint32_t m = 0; m < 4; ++m) {
                        bool reqValid = false;
                        uint64_t reqData = 0;
                        if ( !mcReqQueue[m].IsEmpty()) {
                            reqValid = true;
                            reqData = mcReqQueue[m].Front();
                        }
                        lastMcReqPresented[m] = reqValid;

                        const bool respReady = !mcRespQueue[m].IsFull();

                        out.Push( reqValid);
                        out.Push( reqData);
                        out.Push( respReady);
                    }

                    in = co_yield out;
                }
            }
        );

        // Cache PortIds
        _KlRxValidIn   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 0); });
        _KlRxDataIn    = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 1); });
        _KlTxReadyIn   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 2); });
        _McReqReadyIn  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 0); });
        _McRespValidIn = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 1); });
        _McRespDataIn  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 2); });

        _KlRxReadyOut  = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 0); });
        _KlTxValidOut  = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 1); });
        _KlTxDataOut   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 2); });
        _McReqValidOut = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 0); });
        _McReqDataOut  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 1); });
        _McRespReadyOut = silo::Buff< rube::PortId>( 4, [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 2); });
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr uint32_t DieId( void) const noexcept { return _DieId; }

    rube::PortId KlRxValid( uint32_t t) const noexcept { return _KlRxValidIn[t]; }
    rube::PortId KlRxData( uint32_t t) const noexcept  { return _KlRxDataIn[t]; }
    rube::PortId KlTxReady( uint32_t t) const noexcept { return _KlTxReadyIn[t]; }

    rube::PortId KlRxReady( uint32_t t) const noexcept { return _KlRxReadyOut[t]; }
    rube::PortId KlTxValid( uint32_t t) const noexcept { return _KlTxValidOut[t]; }
    rube::PortId KlTxData( uint32_t t) const noexcept  { return _KlTxDataOut[t]; }

    rube::PortId McReqReady( uint32_t m) const noexcept  { return _McReqReadyIn[m]; }
    rube::PortId McRespValid( uint32_t m) const noexcept { return _McRespValidIn[m]; }
    rube::PortId McRespData( uint32_t m) const noexcept  { return _McRespDataIn[m]; }

    rube::PortId McReqValid( uint32_t m) const noexcept  { return _McReqValidOut[m]; }
    rube::PortId McReqData( uint32_t m) const noexcept   { return _McReqDataOut[m]; }
    rube::PortId McRespReady( uint32_t m) const noexcept { return _McRespReadyOut[m]; }

    KarstLink KlLink( uint32_t t) const noexcept
    {
        return KarstLink(
            KlTxValid( t),
            KlTxData( t),
            KlTxReady( t),
            KlRxValid( t),
            KlRxData( t),
            KlRxReady( t)
        );
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

