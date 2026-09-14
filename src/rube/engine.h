// engine.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "heist/atelier.h"
#include "rube/coro_kernel.h"
#include "rube/layout.h"
#include "rube/module.h"
#include "rube/port.h"
#include "rube/trigger.h"
#include "silo/buff.h"

#include <algorithm>
#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------

enum class SimEngineModeKind
{
    Serial,
    Parallel,
};

struct SimEngineMode
{
    SimEngineModeKind   _Kind{SimEngineModeKind::Serial};
    uint32_t            _Workers{0};

    static constexpr SimEngineMode Serial( void) noexcept
    {
        return SimEngineMode{SimEngineModeKind::Serial, 0};
    }

    static constexpr SimEngineMode Parallel( uint32_t workers = 4) noexcept
    {
        return SimEngineMode{SimEngineModeKind::Parallel, workers};
    }
};

//-------------------------------------------------------------------------------------------------
// SimEngine — digital circuit simulation engine supporting Serial and Parallel Drive modes.

class SimEngine
{
public:
    TriggerWad< uint64_t>   _Triggers{};
    silo::Buff< FastWarp>   _FastWarps{};
    silo::Buff< CoroWarp>   _CoroWarps{};
    silo::Buff< TriggerId>  _PortToTrigger{};
    size_t                  _CycleCount{0};
    SimEngineMode           _Mode{SimEngineMode::Serial()};
    PortId                  _ClkPort{};

public:
    constexpr SimEngine( void) noexcept = default;

    static SimEngine Create( const Layout& layout)
    {
        SimEngine engine;
        engine._PortToTrigger = layout.PortToTrigger();
        engine._Triggers = layout.BuildTriggers( engine._PortToTrigger);
        engine._FastWarps = layout.CompileWarps( engine._PortToTrigger);
        engine._CoroWarps = layout.CompileCoroWarps( engine._PortToTrigger);
        engine._CycleCount = 0;
        engine._Mode = SimEngineMode::Serial();
        return engine;
    }

    void WithMode( SimEngineMode mode) noexcept
    {
        _Mode = mode;
    }

    SimEngine& WithClock( PortId clkPort) noexcept
    {
        _ClkPort = clkPort;
        return *this;
    }

    constexpr PortId GetClock( void) const noexcept
    {
        return _ClkPort;
    }

    TriggerId GetPortTrigger( PortId portId) const noexcept
    {
        const uint32_t idx = portId.Index();
        if ( idx >= _PortToTrigger.Size()) return 0xFFFF'FFFF;
        return _PortToTrigger[idx];
    }

    template < typename T = uint64_t>
    T GetTrigger( TriggerId id) const noexcept
    {
        return static_cast< T>( _Triggers.Current( id));
    }

    template < typename T = uint64_t>
    void SetTrigger( TriggerId id, T val, bool isX = false, bool isI = false) noexcept
    {
        _Triggers.SetFuture( id, static_cast< uint64_t>( val), isX, isI);
    }

    template < typename T = uint64_t>
    void SetTriggerImmediate( TriggerId id, T val, bool isX = false, bool isI = false) noexcept
    {
        _Triggers.SetImmediate( id, static_cast< uint64_t>( val), isX, isI);
    }

    template < typename T = uint64_t>
    T Get( PortId portId) const noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return T{0};
        const uint64_t val = _Triggers.Current( trigId);
        if constexpr ( std::is_same_v< T, bool>) {
            return ( val & 1) != 0;
        } else {
            return static_cast< T>( val);
        }
    }

    template < typename T = uint64_t>
    bool Set( PortId portId, T val, bool isX = false, bool isI = false) noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return false;
        if constexpr ( std::is_same_v< T, bool>) {
            SetTriggerImmediate( trigId, val ? 1ULL : 0ULL, isX, isI);
        } else {
            SetTriggerImmediate( trigId, static_cast< uint64_t>( val), isX, isI);
        }
        return true;
    }

    bool IsX( PortId portId) const noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return true;
        return _Triggers.IsX( trigId);
    }

    bool IsI( PortId portId) const noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return false;
        return _Triggers.IsI( trigId);
    }

    bool IsZ( PortId portId) const noexcept
    {
        return IsI( portId);
    }

    bool IsValid( PortId portId) const noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return false;
        return _Triggers.IsValid( trigId);
    }

    // Convenience aliases
    uint64_t GetPortValue( PortId portId) const noexcept { return Get< uint64_t>( portId); }
    bool GetPortBool( PortId portId) const noexcept { return Get< bool>( portId); }
    uint32_t GetPortU32( PortId portId) const noexcept { return Get< uint32_t>( portId); }

    template < typename T>
    bool SetPortValue( PortId portId, T val, bool isX = false, bool isI = false) noexcept { return Set( portId, val, isX, isI); }
    template < typename T>
    bool SetPortBool( PortId portId, T val, bool isX = false, bool isI = false) noexcept { return Set( portId, val, isX, isI); }
    template < typename T>
    bool SetPortU32( PortId portId, T val, bool isX = false, bool isI = false) noexcept { return Set( portId, val, isX, isI); }

    bool IsPortX( PortId portId) const noexcept { return IsX( portId); }
    bool IsPortI( PortId portId) const noexcept { return IsI( portId); }
    bool IsPortZ( PortId portId) const noexcept { return IsZ( portId); }
    bool IsPortValid( PortId portId) const noexcept { return IsValid( portId); }

    static void EvalCoroInstance(
        const CoroCell& coroCell,
        const silo::Buff< TriggerId>& inTriggers,
        const silo::Buff< TriggerId>& outTriggers,
        TriggerWad< uint64_t>& triggers)
    {
        const uint32_t inLen = inTriggers.Size();
        const uint32_t outLen = outTriggers.Size();

        CoroPorts inPorts;
        const uint32_t inCount = ( std::min)( inLen, CORO_MAX_PORTS);
        for ( uint32_t k = 0; k < inCount; ++k) {
            inPorts._Vals[k] = triggers.Current( inTriggers[k]);
        }
        inPorts._Len = inCount;

        if ( coroCell.IsDone()) {
            return;
        }

        const CoroRes res = coroCell.Resume( inPorts);
        if ( res.IsYield()) {
            if ( outLen > 0) {
                const uint32_t outCount = ( std::min)( outLen, res.Ports().Len());
                for ( uint32_t k = 0; k < outCount; ++k) {
                    triggers.SetFuture( outTriggers[k], res.Ports()._Vals[k]);
                }
            }
        }
    }

    size_t Drive( void)
    {
        auto evalWarpLanes = [this]( const FastWarp& warp, uint32_t startLane, uint32_t endLane) {
            const KernelOp op = warp._Op;
            const uint64_t mask = warp._Mask;
            for ( uint32_t l = startLane; l < endLane; ++l) {
                const TriggerId in1Trig = warp._In1[l];
                const TriggerId in2Trig = warp._In2[l];
                const TriggerId outTrig = warp._Out[l];

                const uint64_t in1 = _Triggers._CurrentVals[in1Trig];
                const uint64_t in2 = _Triggers._CurrentVals[in2Trig];
                const uint8_t f1 = _Triggers._Flags[in1Trig];
                const uint8_t f2 = _Triggers._Flags[in2Trig];

                if ( ( ( f1 | f2) & CURR_MASK) == 0) {
                    const uint64_t raw = EvalRaw( op, in1, in2, mask);
                    _Triggers._FutureVals[outTrig] = raw;
                    _Triggers._Flags[outTrig] = static_cast< uint8_t>( _Triggers._Flags[outTrig] & ~FUTR_MASK);
                } else {
                    const bool x1 = ( f1 & CURR_X) != 0;
                    const bool i1 = ( f1 & CURR_I) != 0;
                    const bool x2 = ( f2 & CURR_X) != 0;
                    const bool i2 = ( f2 & CURR_I) != 0;
                    const Eval4Result< uint64_t> res = Eval4State( op, in1, x1, i1, in2, x2, i2, mask);
                    _Triggers._FutureVals[outTrig] = res._Val;
                    uint8_t f = static_cast< uint8_t>( _Triggers._Flags[outTrig] & ~FUTR_MASK);
                    if ( res._IsX) f |= FUTR_X;
                    if ( res._IsI) f |= FUTR_I;
                    _Triggers._Flags[outTrig] = f;
                }
            }
        };

        auto evalCoroWarpLanes = [this]( const CoroWarp& warp, uint32_t startLane, uint32_t endLane) {
            for ( uint32_t l = startLane; l < endLane; ++l) {
                const auto& inTrigs = warp._InTriggers[l];
                const auto& outTrigs = warp._OutTriggers[l];
                const auto& coroCell = warp._Instances[l];
                EvalCoroInstance( coroCell, inTrigs, outTrigs, _Triggers);
            }
        };

        auto& atelier = heist::Atelier::Instance();
        if ( _Mode._Kind == SimEngineModeKind::Parallel && !atelier.IsImmediate() && atelier.SzThreads() > 1) {
            heist::Maestro* mainMaestro = atelier.MainMaestro();
            for ( uint32_t wIdx = 0; wIdx < _FastWarps.Size(); ++wIdx) {
                const FastWarp& warp = _FastWarps[wIdx];
                const uint32_t count = warp._Count;
                const uint32_t chunkSize = 64;
                const uint32_t numChunks = ( count + chunkSize - 1) / chunkSize;
                for ( uint32_t c = 0; c < numChunks; ++c) {
                    const uint32_t start = c * chunkSize;
                    const uint32_t end = ( std::min)( start + chunkSize, count);
                    mainMaestro->PostJob( stalks::WorkPtr::FromLambda(
                        [&warp, evalWarpLanes, start, end]( stalks::IWorker*) {
                            evalWarpLanes( warp, start, end);
                        }
                    ));
                }
            }
            for ( uint32_t wIdx = 0; wIdx < _CoroWarps.Size(); ++wIdx) {
                const CoroWarp& warp = _CoroWarps[wIdx];
                const uint32_t count = warp._Count;
                const uint32_t chunkSize = 64;
                const uint32_t numChunks = ( count + chunkSize - 1) / chunkSize;
                for ( uint32_t c = 0; c < numChunks; ++c) {
                    const uint32_t start = c * chunkSize;
                    const uint32_t end = ( std::min)( start + chunkSize, count);
                    mainMaestro->PostJob( stalks::WorkPtr::FromLambda(
                        [&warp, evalCoroWarpLanes, start, end]( stalks::IWorker*) {
                            evalCoroWarpLanes( warp, start, end);
                        }
                    ));
                }
            }
            atelier.DoLaunch();
        } else {
            for ( uint32_t wIdx = 0; wIdx < _FastWarps.Size(); ++wIdx) {
                evalWarpLanes( _FastWarps[wIdx], 0, _FastWarps[wIdx]._Count);
            }
            for ( uint32_t wIdx = 0; wIdx < _CoroWarps.Size(); ++wIdx) {
                evalCoroWarpLanes( _CoroWarps[wIdx], 0, _CoroWarps[wIdx]._Count);
            }
        }

        _Triggers.AdvanceAll();
        _CycleCount += 1;
        return _CycleCount;
    }

    uint32_t Settle( uint32_t maxCycles = 100)
    {
        uint32_t cycles = 0;
        while ( cycles < maxCycles) {
            Drive();
            cycles += 1;

            bool anyEdge = false;
            for ( uint32_t t = 0; t < _Triggers.Size(); ++t) {
                if ( _Triggers.IsEdge( t)) {
                    anyEdge = true;
                    break;
                }
            }
            if ( !anyEdge) {
                break;
            }
        }
        return cycles;
    }

    uint32_t Advance( PortId clkPort, uint32_t ticks = 1, uint32_t maxSettleCycles = 100)
    {
        if ( !clkPort.IsValid()) {
            return 0;
        }

        uint32_t totalCycles = 0;
        for ( uint32_t i = 0; i < ticks; ++i) {
            const bool baseline = Get< bool>( clkPort);

            Set( clkPort, !baseline);
            totalCycles += Settle( maxSettleCycles);

            Set( clkPort, baseline);
            totalCycles += Settle( maxSettleCycles);
        }
        return totalCycles;
    }

    uint32_t AdvanceTrigger( TriggerId clkTrig, uint32_t ticks = 1, uint32_t maxSettleCycles = 100)
    {
        if ( clkTrig == 0xFFFF'FFFF || clkTrig >= _Triggers.Size()) {
            return 0;
        }

        uint32_t totalCycles = 0;
        for ( uint32_t i = 0; i < ticks; ++i) {
            const bool baseline = ( GetTrigger< uint64_t>( clkTrig) & 1) != 0;

            SetTriggerImmediate( clkTrig, baseline ? 0ULL : 1ULL);
            totalCycles += Settle( maxSettleCycles);

            SetTriggerImmediate( clkTrig, baseline ? 1ULL : 0ULL);
            totalCycles += Settle( maxSettleCycles);
        }
        return totalCycles;
    }

    uint32_t Advance( uint32_t ticks = 1, uint32_t maxSettleCycles = 100)
    {
        if ( !_ClkPort.IsValid()) {
            return 0;
        }
        return Advance( _ClkPort, ticks, maxSettleCycles);
    }
};

} // namespace trellis::rube
