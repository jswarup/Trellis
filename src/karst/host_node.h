// host_node.h ----------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "karst/link.h"
#include "rube/rube.h"
#include "silo/arr.h"
#include "silo/fifo.h"
#include "stalks/atm.h"

#include <cstdint>
#include <deque>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// Host request transaction.

struct HostTransaction
{
    uint32_t            _Addr{0};
    uint32_t            _Data{0};
    bool                _IsWrite{false};
};

//-------------------------------------------------------------------------------------------------
// Host read response.

struct HostResponse
{
    uint32_t            _Addr{0};
    uint32_t            _Data{0};
};

//-------------------------------------------------------------------------------------------------
// Host performance metrics.

struct HostStats
{
    uint32_t            _TxCount{0};
    uint32_t            _RxCount{0};
    uint32_t            _WritesPosted{0};
    uint32_t            _ReadsPosted{0};
};

//-------------------------------------------------------------------------------------------------
// Shared state for a host node.

struct HostNodeState
{
    mutable stalks::Spinlock        _Lock{};
    std::deque< HostTransaction>    _TxQueue{};
    std::deque< HostResponse>       _RxQueue{};
    HostStats                       _Stats{};
};

//-------------------------------------------------------------------------------------------------
// KarstHostNode — KarstFore front-port IO chiplet coroutine model.
// Exposes primary and cross-home KarstLink links (Link0 and Link1).

class KarstHostNode
{
private:
    rube::ModuleId                  _Id{};
    uint32_t                        _HostId{0};
    std::shared_ptr< HostNodeState> _State{};

    rube::PortId                    _L0TxReadyIn{};
    rube::PortId                    _L0RxValidIn{};
    rube::PortId                    _L0RxDataIn{};
    rube::PortId                    _L1TxReadyIn{};
    rube::PortId                    _L1RxValidIn{};
    rube::PortId                    _L1RxDataIn{};

    rube::PortId                    _L0TxValidOut{};
    rube::PortId                    _L0TxDataOut{};
    rube::PortId                    _L0RxReadyOut{};
    rube::PortId                    _L1TxValidOut{};
    rube::PortId                    _L1TxDataOut{};
    rube::PortId                    _L1RxReadyOut{};

public:
    KarstHostNode( void) = default;

    KarstHostNode(
        rube::Layout& layout,
        const char* name,
        uint32_t hostId,
        rube::ModuleId parent = rube::ModuleId{})
        : _HostId( hostId),
          _State( std::make_shared< HostNodeState>())
    {
        rube::PortDesc inDescs[6] = {
            rube::PortDesc( "L0TxReady", rube::PortType::Bool()),
            rube::PortDesc( "L0RxValid", rube::PortType::Bool()),
            rube::PortDesc( "L0RxData",  rube::PortType::U64Val()),
            rube::PortDesc( "L1TxReady", rube::PortType::Bool()),
            rube::PortDesc( "L1RxValid", rube::PortType::Bool()),
            rube::PortDesc( "L1RxData",  rube::PortType::U64Val())
        };

        rube::PortDesc outDescs[6] = {
            rube::PortDesc( "L0TxValid", rube::PortType::Bool()),
            rube::PortDesc( "L0TxData",  rube::PortType::U64Val()),
            rube::PortDesc( "L0RxReady", rube::PortType::Bool()),
            rube::PortDesc( "L1TxValid", rube::PortType::Bool()),
            rube::PortDesc( "L1TxData",  rube::PortType::U64Val()),
            rube::PortDesc( "L1RxReady", rube::PortType::Bool())
        };

        std::shared_ptr< HostNodeState> state = _State;

        _Id = layout.AddCoroModule(
            name,
            parent,
            silo::Arr< const rube::PortDesc>( inDescs, 6),
            silo::Arr< const rube::PortDesc>( outDescs, 6),
            [hostId, state]() -> rube::CoroTask {
                silo::Fifo< HostTransaction, 16> activeQueue;
                bool lastL0TxPresented = false;
                bool lastL1TxPresented = false;
                bool presentedIsL1 = false;

                rube::CoroPorts in = co_await rube::CoroIn{};

                while ( true)
                {
                    // 1. Check completion of previous TX
                    if ( lastL0TxPresented && in.Get< bool>( 0) && !activeQueue.IsEmpty() && !presentedIsL1) {
                        activeQueue.PopFront();
                        auto lock = state->_Lock.Lock();
                        state->_Stats._TxCount++;
                    }
                    if ( lastL1TxPresented && in.Get< bool>( 3) && !activeQueue.IsEmpty() && presentedIsL1) {
                        activeQueue.PopFront();
                        auto lock = state->_Lock.Lock();
                        state->_Stats._TxCount++;
                    }

                    // 2. Sample incoming responses from Link0
                    if ( in.Get< bool>( 1)) {
                        const KarstFlit flit = KarstFlit::Unpack( in[2]);
                        auto lock = state->_Lock.Lock();
                        state->_RxQueue.push_back( HostResponse{flit._Addr, flit._Data});
                        state->_Stats._RxCount++;
                    }

                    // 3. Sample incoming responses from Link1
                    if ( in.Get< bool>( 4)) {
                        const KarstFlit flit = KarstFlit::Unpack( in[5]);
                        auto lock = state->_Lock.Lock();
                        state->_RxQueue.push_back( HostResponse{flit._Addr, flit._Data});
                        state->_Stats._RxCount++;
                    }

                    // 4. Fetch new transactions from shared state queue
                    {
                        auto lock = state->_Lock.Lock();
                        while ( !state->_TxQueue.empty() && !activeQueue.IsFull()) {
                            activeQueue.PushBack( state->_TxQueue.front());
                            state->_TxQueue.pop_front();
                        }
                    }

                    // 5. Present outgoing request
                    bool l0TxValid = false;
                    uint64_t l0TxData = 0;
                    bool l1TxValid = false;
                    uint64_t l1TxData = 0;

                    if ( !activeQueue.IsEmpty()) {
                        const HostTransaction& tx = activeQueue.Front();
                        const uint64_t raw = KarstFlit::Pack( tx._Addr, tx._Data, static_cast< uint8_t>( hostId), tx._IsWrite);

                        // Karst dual-homing FE routing:
                        // Host 0..3: Primary to Die 0 (Link0), cross-home to Die 1 (Link1)
                        // Host 4..7: Primary to Die 1 (Link0), cross-home to Die 0 (Link1)
                        const uint32_t targetDie = ( tx._Addr >> 12) & 1U;
                        const uint32_t primaryDie = ( hostId < 4) ? 0U : 1U;

                        if ( targetDie == primaryDie) {
                            l0TxValid = true;
                            l0TxData = raw;
                            presentedIsL1 = false;
                        } else {
                            l1TxValid = true;
                            l1TxData = raw;
                            presentedIsL1 = true;
                        }
                    }

                    lastL0TxPresented = l0TxValid;
                    lastL1TxPresented = l1TxValid;

                    rube::CoroPorts out;
                    out.Push( l0TxValid);
                    out.Push( l0TxData);
                    out.Push( true);                            // L0RxReady
                    out.Push( l1TxValid);
                    out.Push( l1TxData);
                    out.Push( true);                            // L1RxReady

                    in = co_yield out;
                }
            }
        );

        _L0TxReadyIn  = layout.InPort( _Id, 0);
        _L0RxValidIn  = layout.InPort( _Id, 1);
        _L0RxDataIn   = layout.InPort( _Id, 2);
        _L1TxReadyIn  = layout.InPort( _Id, 3);
        _L1RxValidIn  = layout.InPort( _Id, 4);
        _L1RxDataIn   = layout.InPort( _Id, 5);

        _L0TxValidOut = layout.OutPort( _Id, 0);
        _L0TxDataOut  = layout.OutPort( _Id, 1);
        _L0RxReadyOut = layout.OutPort( _Id, 2);
        _L1TxValidOut = layout.OutPort( _Id, 3);
        _L1TxDataOut  = layout.OutPort( _Id, 4);
        _L1RxReadyOut = layout.OutPort( _Id, 5);
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr uint32_t HostId( void) const noexcept { return _HostId; }

    KarstLink Link0( void) const noexcept
    {
        return KarstLink(
            _L0TxValidOut,
            _L0TxDataOut,
            _L0TxReadyIn,
            _L0RxValidIn,
            _L0RxDataIn,
            _L0RxReadyOut
        );
    }

    KarstLink Link1( void) const noexcept
    {
        return KarstLink(
            _L1TxValidOut,
            _L1TxDataOut,
            _L1TxReadyIn,
            _L1RxValidIn,
            _L1RxDataIn,
            _L1RxReadyOut
        );
    }

    void PostWrite( uint32_t addr, uint32_t data)
    {
        if ( !_State) return;
        auto lock = _State->_Lock.Lock();
        _State->_TxQueue.push_back( HostTransaction{addr, data, true});
        _State->_Stats._WritesPosted++;
    }

    void PostRead( uint32_t addr)
    {
        if ( !_State) return;
        auto lock = _State->_Lock.Lock();
        _State->_TxQueue.push_back( HostTransaction{addr, 0, false});
        _State->_Stats._ReadsPosted++;
    }

    bool HasResponses( void) const
    {
        if ( !_State) return false;
        auto lock = _State->_Lock.Lock();
        return !_State->_RxQueue.empty();
    }

    bool PopResponse( HostResponse& out)
    {
        if ( !_State) return false;
        auto lock = _State->_Lock.Lock();
        if ( _State->_RxQueue.empty()) return false;
        out = _State->_RxQueue.front();
        _State->_RxQueue.pop_front();
        return true;
    }

    HostStats Stats( void) const
    {
        if ( !_State) return HostStats{};
        auto lock = _State->_Lock.Lock();
        return _State->_Stats;
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

