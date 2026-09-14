// noc.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "karst/link.h"
#include "rube/rube.h"
#include "silo/arr.h"
#include "silo/buff.h"

#include <cstdint>
#include <deque>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstNoc — KarstHind MF-NoC crossbar switch coroutine.
// Switches 10 KarstLink ports (TW0..TW9) and 4 local DDR5 Memory Controllers (MC0..MC3).
// Implements address decoding, 1 kB striped memory interleaving, and inter-die KarstLink routing.

class KarstNoc
{
private:
    rube::ModuleId      _Id{};
    uint32_t            _DieId{0};

    // Cached port IDs for fast accessor lookups
    silo::Buff< rube::PortId> _TwRxValidIn{};
    silo::Buff< rube::PortId> _TwRxDataIn{};
    silo::Buff< rube::PortId> _TwTxReadyIn{};
    silo::Buff< rube::PortId> _McReqReadyIn{};
    silo::Buff< rube::PortId> _McRespValidIn{};
    silo::Buff< rube::PortId> _McRespDataIn{};

    silo::Buff< rube::PortId> _TwRxReadyOut{};
    silo::Buff< rube::PortId> _TwTxValidOut{};
    silo::Buff< rube::PortId> _TwTxDataOut{};
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
        //   [0..29]:  10 TW ports x (TwRxValid, TwRxData, TwTxReady)
        //   [30..41]: 4 MC ports  x (McReqReady, McRespValid, McRespData)
        silo::Buff< rube::PortDesc> inDescs( 42, []( uint32_t) {
            return rube::PortDesc{};
        });
        for ( uint32_t t = 0; t < 10; ++t) {
            const std::string pfx = "Tw" + std::to_string( t);
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
        //   [0..29]:  10 TW ports x (TwRxReady, TwTxValid, TwTxData)
        //   [30..41]: 4 MC ports  x (McReqValid, McReqData, McRespReady)
        silo::Buff< rube::PortDesc> outDescs( 42, []( uint32_t) {
            return rube::PortDesc{};
        });
        for ( uint32_t t = 0; t < 10; ++t) {
            const std::string pfx = "Tw" + std::to_string( t);
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
                std::deque< uint64_t> mcReqQueue[4];
                std::deque< uint64_t> twTxQueue[10];
                const size_t capacity = 16;

                bool lastMcReqPresented[4] = {false, false, false, false};
                bool lastTwTxPresented[10] = {};

                rube::CoroPorts in = co_await rube::CoroIn{};

                while ( true)
                {
                    // 1. Retire successfully transferred outputs
                    for ( uint32_t m = 0; m < 4; ++m) {
                        const bool reqReady = in.Get< bool>( 30 + 3 * m + 0);
                        if ( lastMcReqPresented[m] && reqReady && !mcReqQueue[m].empty()) {
                            mcReqQueue[m].pop_front();
                        }
                    }
                    for ( uint32_t t = 0; t < 10; ++t) {
                        const bool txReady = in.Get< bool>( 3 * t + 2);
                        if ( lastTwTxPresented[t] && txReady && !twTxQueue[t].empty()) {
                            twTxQueue[t].pop_front();
                        }
                    }

                    // 2. Ingress from KarstLink links (TW0..TW9)
                    for ( uint32_t t = 0; t < 10; ++t) {
                        const bool rxValid = in.Get< bool>( 3 * t + 0);
                        if ( rxValid) {
                            const uint64_t raw = in[3 * t + 1];
                            const KarstFlit flit = KarstFlit::Unpack( raw);

                            // Decode destination die from address bit 12
                            const uint32_t targetDie = ( flit._Addr >> 12) & 1U;
                            if ( targetDie == dieId) {
                                // Local MC on this die: select MC from address bits [11:10] (1 kB interleaving)
                                const uint32_t mcIdx = ( flit._Addr >> 10) & 3U;
                                if ( mcReqQueue[mcIdx].size() < capacity) {
                                    mcReqQueue[mcIdx].push_back( raw);
                                }
                            } else {
                                // Peer die: route out via inter-die KarstLink port (port 8)
                                if ( twTxQueue[8].size() < capacity) {
                                    twTxQueue[8].push_back( raw);
                                }
                            }
                        }
                    }

                    // 3. Ingress from Memory Controller responses (MC0..MC3)
                    for ( uint32_t m = 0; m < 4; ++m) {
                        const bool respValid = in.Get< bool>( 30 + 3 * m + 1);
                        if ( respValid) {
                            const uint64_t raw = in[30 + 3 * m + 2];
                            const KarstFlit flit = KarstFlit::Unpack( raw);

                            // Route response back to requester based on srcId and dieId
                            uint32_t targetTw = 0;
                            if ( dieId == 0) {
                                targetTw = flit._SrcId % 8U;
                            } else {
                                targetTw = ( ( flit._SrcId >= 4) ? ( flit._SrcId - 4) : ( flit._SrcId + 4)) % 8U;
                            }

                            if ( twTxQueue[targetTw].size() < capacity) {
                                twTxQueue[targetTw].push_back( raw);
                            }
                        }
                    }

                    // 4. Drive outputs
                    rube::CoroPorts out;

                    // 4a. 10 TW ports
                    for ( uint32_t t = 0; t < 10; ++t) {
                        const bool rxReady = ( twTxQueue[t].size() < capacity);
                        bool txValid = false;
                        uint64_t txData = 0;
                        if ( !twTxQueue[t].empty()) {
                            txValid = true;
                            txData = twTxQueue[t].front();
                        }
                        lastTwTxPresented[t] = txValid;

                        out.Push( rxReady);
                        out.Push( txValid);
                        out.Push( txData);
                    }

                    // 4b. 4 MC ports
                    for ( uint32_t m = 0; m < 4; ++m) {
                        bool reqValid = false;
                        uint64_t reqData = 0;
                        if ( !mcReqQueue[m].empty()) {
                            reqValid = true;
                            reqData = mcReqQueue[m].front();
                        }
                        lastMcReqPresented[m] = reqValid;

                        const bool respReady = true;

                        out.Push( reqValid);
                        out.Push( reqData);
                        out.Push( respReady);
                    }

                    in = co_yield out;
                }
            }
        );

        // Cache PortIds
        _TwRxValidIn   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 0); });
        _TwRxDataIn    = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 1); });
        _TwTxReadyIn   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.InPort( _Id, 3 * t + 2); });
        _McReqReadyIn  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 0); });
        _McRespValidIn = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 1); });
        _McRespDataIn  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.InPort( _Id, 30 + 3 * m + 2); });

        _TwRxReadyOut  = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 0); });
        _TwTxValidOut  = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 1); });
        _TwTxDataOut   = silo::Buff< rube::PortId>( 10, [&]( uint32_t t) { return layout.OutPort( _Id, 3 * t + 2); });
        _McReqValidOut = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 0); });
        _McReqDataOut  = silo::Buff< rube::PortId>( 4,  [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 1); });
        _McRespReadyOut = silo::Buff< rube::PortId>( 4, [&]( uint32_t m) { return layout.OutPort( _Id, 30 + 3 * m + 2); });
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr uint32_t DieId( void) const noexcept { return _DieId; }

    rube::PortId TwRxValid( uint32_t t) const noexcept { return _TwRxValidIn[t]; }
    rube::PortId TwRxData( uint32_t t) const noexcept  { return _TwRxDataIn[t]; }
    rube::PortId TwTxReady( uint32_t t) const noexcept { return _TwTxReadyIn[t]; }

    rube::PortId TwRxReady( uint32_t t) const noexcept { return _TwRxReadyOut[t]; }
    rube::PortId TwTxValid( uint32_t t) const noexcept { return _TwTxValidOut[t]; }
    rube::PortId TwTxData( uint32_t t) const noexcept  { return _TwTxDataOut[t]; }

    rube::PortId McReqReady( uint32_t m) const noexcept  { return _McReqReadyIn[m]; }
    rube::PortId McRespValid( uint32_t m) const noexcept { return _McRespValidIn[m]; }
    rube::PortId McRespData( uint32_t m) const noexcept  { return _McRespDataIn[m]; }

    rube::PortId McReqValid( uint32_t m) const noexcept  { return _McReqValidOut[m]; }
    rube::PortId McReqData( uint32_t m) const noexcept   { return _McReqDataOut[m]; }
    rube::PortId McRespReady( uint32_t m) const noexcept { return _McRespReadyOut[m]; }

    KarstLink TwLink( uint32_t t) const noexcept
    {
        return KarstLink(
            TwTxValid( t),
            TwTxData( t),
            TwTxReady( t),
            TwRxValid( t),
            TwRxData( t),
            TwRxReady( t)
        );
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

