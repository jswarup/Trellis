// vm_adaptor.h -----------------------------------------------------------------------------------------------------
#pragma once

#include "crew/node.h"
#include "crew/protocol.h"
#include "crew/vm_runner.h"
#include "rube/rube.h"
#include "silo/arr.h"

#include <cstdint>
#include <deque>
#include <memory>
#include <string>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------
// Convenience macros for VM coroutine guest MMIO execution.

#define VM_MMIO_WRITE( inPorts, addr, val)                               \
    do {                                                                 \
        inPorts = co_yield trellis::crew::VmBus::Write( addr, val);      \
        while ( !inPorts[0].IsTrue()) {                                  \
            inPorts = co_yield trellis::crew::VmBus::Write( addr, val);  \
        }                                                                \
        inPorts = co_yield trellis::crew::VmBus::Idle();                 \
    } while ( 0)

#define VM_MMIO_READ( inPorts, addr, outVal)                             \
    do {                                                                 \
        inPorts = co_yield trellis::crew::VmBus::Read( addr);            \
        while ( !inPorts[0].IsTrue()) {                                  \
            inPorts = co_yield trellis::crew::VmBus::Read( addr);        \
        }                                                                \
        outVal = static_cast< uint32_t>( inPorts[1].Val());              \
        inPorts = co_yield trellis::crew::VmBus::Idle();                 \
    } while ( 0)

//-------------------------------------------------------------------------------------------------
// VMAdaptor — hardware adapter connecting a VMRunner guest to the Rube inter-VM network.
// Implements MMIO registers (REG_NODE_ID, REG_STATUS, REG_TX_DATA, REG_RX_DATA, REG_RX_COUNT)
// and bridges them to valid/ready streaming links across Rube netlist interconnects.

class VMAdaptor
{
private:
    rube::ModuleId              _Id{};
    uint32_t                    _NodeId{0};
    std::unique_ptr< NodeStats> _Stats{};

    // Local VM bus ports
    rube::PortId                _VmReqIn{};
    rube::PortId                _VmWriteIn{};
    rube::PortId                _VmAddrIn{};
    rube::PortId                _VmWDataIn{};
    rube::PortId                _VmAckOut{};
    rube::PortId                _VmRDataOut{};

    // Inter-VM link ports
    rube::PortId                _LinkRxValidIn{};
    rube::PortId                _LinkRxDataIn{};
    rube::PortId                _LinkTxReadyIn{};
    rube::PortId                _LinkTxValidOut{};
    rube::PortId                _LinkTxDataOut{};
    rube::PortId                _LinkRxReadyOut{};

public:
    VMAdaptor( void) = default;

    VMAdaptor(
        rube::Layout& layout,
        const char* name,
        uint32_t nodeId,
        const VMRunner* attachedVm = nullptr,
        rube::ModuleId parent = rube::ModuleId{})
        : _NodeId( nodeId)
        , _Stats( std::make_unique< NodeStats>())
    {
        rube::PortDesc inDescs[7] = {
            rube::PortDesc( "VmReq",        rube::PortType::Bool()),
            rube::PortDesc( "VmWrite",      rube::PortType::Bool()),
            rube::PortDesc( "VmAddr",       rube::PortType::U32Val()),
            rube::PortDesc( "VmWData",      rube::PortType::U32Val()),
            rube::PortDesc( "LinkRxValid",  rube::PortType::Bool()),
            rube::PortDesc( "LinkRxData",   rube::PortType::U32Val()),
            rube::PortDesc( "LinkTxReady",  rube::PortType::Bool())
        };

        rube::PortDesc outDescs[5] = {
            rube::PortDesc( "VmAck",        rube::PortType::Bool()),
            rube::PortDesc( "VmRData",      rube::PortType::U32Val()),
            rube::PortDesc( "LinkTxValid",  rube::PortType::Bool()),
            rube::PortDesc( "LinkTxData",   rube::PortType::U32Val()),
            rube::PortDesc( "LinkRxReady",  rube::PortType::Bool())
        };

        NodeStats* stats = _Stats.get();

        _Id = layout.AddCoroModule(
            name,
            parent,
            silo::Arr< const rube::PortDesc>( inDescs, 7),
            silo::Arr< const rube::PortDesc>( outDescs, 5),
            [nodeId, stats]() -> rube::CoroTask {
                std::deque< uint8_t> rxQueue;
                std::deque< uint8_t> txQueue;
                const uint32_t capacity = 128;
                bool wasReq = false;
                bool lastTxPresented = false;

                uint32_t lastRData = 0;

                rube::CoroPorts in = co_await rube::CoroIn{};

                while ( true)
                {
                    // 1. Link TX completion (if peer asserted ready, the byte was transferred)
                    if ( lastTxPresented && in[6].IsTrue() && !txQueue.empty()) {
                        txQueue.pop_front();
                        if ( stats) {
                            stats->_BytesSent++;
                        }
                    }

                    // 2. Link RX sampling (if peer asserted valid and we had room)
                    if ( in[4].IsTrue() && rxQueue.size() < capacity) {
                        const uint8_t inByte = static_cast< uint8_t>( in[5].Val() & 0xFF);
                        rxQueue.push_back( inByte);
                        if ( stats) {
                            stats->_BytesReceived++;
                        }
                    }

                    // 3. VM MMIO processing
                    bool vmAck = false;
                    uint32_t vmRData = 0;

                    if ( in[0].IsTrue()) {
                        if ( !wasReq) {
                            wasReq = true;
                            const bool isWrite = in[1].IsTrue();
                            const uint32_t addr = static_cast< uint32_t>( in[2].Val() & 0xFFFF);
                            const uint32_t wdata = static_cast< uint32_t>( in[3].Val());

                            if ( isWrite) {
                                if ( stats) {
                                    stats->_WritesServiced++;
                                }
                                if ( addr == REG_TX_DATA && txQueue.size() < capacity) {
                                    txQueue.push_back( static_cast< uint8_t>( wdata & 0xFF));
                                }
                                vmAck = true;
                                vmRData = 0;
                            } else {
                                if ( stats) {
                                    stats->_ReadsServiced++;
                                }
                                vmAck = true;
                                if ( addr == REG_NODE_ID) {
                                    vmRData = nodeId;
                                } else if ( addr == REG_STATUS) {
                                    uint32_t st = STATUS_PEER_UP;
                                    if ( txQueue.size() < capacity) st |= STATUS_TX_READY;
                                    if ( !rxQueue.empty()) st |= STATUS_RX_READY;
                                    vmRData = st;
                                } else if ( addr == REG_RX_COUNT) {
                                    vmRData = static_cast< uint32_t>( rxQueue.size());
                                } else if ( addr == REG_RX_DATA) {
                                    if ( !rxQueue.empty()) {
                                        vmRData = rxQueue.front();
                                        rxQueue.pop_front();
                                    } else {
                                        vmRData = 0;
                                    }
                                } else {
                                    vmRData = 0;
                                }
                            }
                            lastRData = vmRData;
                        } else {
                            // Request is still held high by VM
                            vmAck = true;
                            vmRData = lastRData;
                        }
                    } else {
                        wasReq = false;
                        vmAck = false;
                        vmRData = 0;
                        lastRData = 0;
                    }

                    // 4. Link TX presentation
                    bool linkTxValid = false;
                    uint32_t linkTxData = 0;
                    if ( !txQueue.empty()) {
                        linkTxValid = true;
                        linkTxData = txQueue.front();
                    }
                    lastTxPresented = linkTxValid;

                    // 5. Link RX readiness
                    const bool linkRxReady = ( rxQueue.size() < capacity);

                    // 6. Yield outputs
                    rube::CoroPorts out;
                    out.Push( vmAck ? rube::Reg::TRUE : rube::Reg::FALSE);
                    out.Push( rube::Reg::Known( vmRData));
                    out.Push( linkTxValid ? rube::Reg::TRUE : rube::Reg::FALSE);
                    out.Push( rube::Reg::Known( linkTxData));
                    out.Push( linkRxReady ? rube::Reg::TRUE : rube::Reg::FALSE);

                    in = co_yield out;
                }
            }
        );

        _VmReqIn        = layout.InPort( _Id, 0);
        _VmWriteIn      = layout.InPort( _Id, 1);
        _VmAddrIn       = layout.InPort( _Id, 2);
        _VmWDataIn      = layout.InPort( _Id, 3);
        _LinkRxValidIn  = layout.InPort( _Id, 4);
        _LinkRxDataIn   = layout.InPort( _Id, 5);
        _LinkTxReadyIn  = layout.InPort( _Id, 6);

        _VmAckOut       = layout.OutPort( _Id, 0);
        _VmRDataOut     = layout.OutPort( _Id, 1);
        _LinkTxValidOut = layout.OutPort( _Id, 2);
        _LinkTxDataOut  = layout.OutPort( _Id, 3);
        _LinkRxReadyOut = layout.OutPort( _Id, 4);

        if ( attachedVm) {
            ConnectVm( layout, *attachedVm);
        }
    }

    void ConnectVm( rube::Layout& layout, const VMRunner& vm)
    {
        layout.Connect( vm.Req(),   _VmReqIn);
        layout.Connect( vm.Write(), _VmWriteIn);
        layout.Connect( vm.Addr(),  _VmAddrIn);
        layout.Connect( vm.WData(), _VmWDataIn);
        layout.Connect( _VmAckOut,   vm.Ack());
        layout.Connect( _VmRDataOut, vm.RData());
    }

    static void Connect( rube::Layout& layout, VMAdaptor& a, VMAdaptor& b)
    {
        // a -> b
        layout.Connect( a.LinkTxValid(), b.LinkRxValid());
        layout.Connect( a.LinkTxData(),  b.LinkRxData());
        layout.Connect( b.LinkRxReady(), a.LinkTxReady());

        // b -> a
        layout.Connect( b.LinkTxValid(), a.LinkRxValid());
        layout.Connect( b.LinkTxData(),  a.LinkRxData());
        layout.Connect( a.LinkRxReady(), b.LinkTxReady());
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr uint32_t NodeId( void) const noexcept { return _NodeId; }

    constexpr rube::PortId VmReq( void) const noexcept { return _VmReqIn; }
    constexpr rube::PortId VmWrite( void) const noexcept { return _VmWriteIn; }
    constexpr rube::PortId VmAddr( void) const noexcept { return _VmAddrIn; }
    constexpr rube::PortId VmWData( void) const noexcept { return _VmWDataIn; }
    constexpr rube::PortId VmAck( void) const noexcept { return _VmAckOut; }
    constexpr rube::PortId VmRData( void) const noexcept { return _VmRDataOut; }

    constexpr rube::PortId LinkRxValid( void) const noexcept { return _LinkRxValidIn; }
    constexpr rube::PortId LinkRxData( void) const noexcept { return _LinkRxDataIn; }
    constexpr rube::PortId LinkTxReady( void) const noexcept { return _LinkTxReadyIn; }
    constexpr rube::PortId LinkTxValid( void) const noexcept { return _LinkTxValidOut; }
    constexpr rube::PortId LinkTxData( void) const noexcept { return _LinkTxDataOut; }
    constexpr rube::PortId LinkRxReady( void) const noexcept { return _LinkRxReadyOut; }

    NodeStats GetStats( void) const noexcept
    {
        return _Stats ? *_Stats : NodeStats{};
    }
};

using VMAdapter = VMAdaptor;

//-------------------------------------------------------------------------------------------------
} // namespace trellis::crew
