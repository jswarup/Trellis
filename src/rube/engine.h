#ifndef TRELLIS_RUBE_ENGINE_H
#define TRELLIS_RUBE_ENGINE_H

//-------------------------------------------------------------------------------------------------

#include "heist/atelier.h"
#include "rube/layout.h"
#include "rube/module.h"
#include "rube/port.h"
#include "rube/reg.h"
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
    silo::Buff< TriggerId>  _PortToTrigger{};
    size_t                  _CycleCount{0};
    SimEngineMode           _Mode{SimEngineMode::Serial()};

public:
    constexpr SimEngine( void) noexcept = default;

    static SimEngine Create( const Layout& layout)
    {
        SimEngine engine;
        engine._PortToTrigger = layout.PortToTrigger();
        engine._Triggers = layout.BuildTriggers( engine._PortToTrigger);
        engine._FastWarps = layout.CompileWarps( engine._PortToTrigger);
        engine._CycleCount = 0;
        engine._Mode = SimEngineMode::Serial();
        return engine;
    }

    void WithMode( SimEngineMode mode) noexcept
    {
        _Mode = mode;
    }

    TriggerId GetPortTrigger( PortId portId) const noexcept
    {
        const uint32_t idx = portId.Index();
        if ( idx >= _PortToTrigger.Size()) return 0xFFFF'FFFF;
        return _PortToTrigger[idx];
    }

    Reg GetTrigger( TriggerId id) const noexcept
    {
        return _Triggers.Current( id);
    }

    void SetTrigger( TriggerId id, Reg val) noexcept
    {
        _Triggers.SetFuture( id, val);
    }

    void SetTriggerImmediate( TriggerId id, Reg val) noexcept
    {
        _Triggers.SetImmediate( id, val);
    }

    Reg GetPortValue( PortId portId) const noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return Reg::Unknown();
        return GetTrigger( trigId);
    }

    bool SetPortValue( PortId portId, Reg val) noexcept
    {
        const TriggerId trigId = GetPortTrigger( portId);
        if ( trigId == 0xFFFF'FFFF) return false;
        SetTriggerImmediate( trigId, val);
        return true;
    }

    Reg GetPortBool( PortId portId) const noexcept
    {
        return GetPortValue( portId).AsBool();
    }

    bool SetPortBool( PortId portId, Reg val) noexcept
    {
        return SetPortValue( portId, val.AsBool());
    }

    Reg GetPortU32( PortId portId) const noexcept
    {
        return GetPortValue( portId).Masked( 0xFFFF'FFFF);
    }

    bool SetPortU32( PortId portId, Reg val) noexcept
    {
        return SetPortValue( portId, val.Masked( 0xFFFF'FFFF));
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
                    const Reg r1{in1, ( f1 & CURR_X) != 0, ( f1 & CURR_I) != 0};
                    const Reg r2{in2, ( f2 & CURR_X) != 0, ( f2 & CURR_I) != 0};
                    const Reg res = Eval( op, r1, r2, mask);
                    _Triggers._FutureVals[outTrig] = res._Val;
                    uint8_t f = static_cast< uint8_t>( _Triggers._Flags[outTrig] & ~FUTR_MASK);
                    if ( res.IsX()) f |= FUTR_X;
                    if ( res.IsI()) f |= FUTR_I;
                    _Triggers._Flags[outTrig] = f;
                }
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
                    const uint32_t end = std::min( start + chunkSize, count);
                    mainMaestro->PostJob( stalks::WorkPtr::FromLambda(
                        [&warp, evalWarpLanes, start, end]( stalks::IWorker*) {
                            evalWarpLanes( warp, start, end);
                        }
                    ));
                }
            }
            atelier.DoLaunch();
        } else {
            for ( uint32_t wIdx = 0; wIdx < _FastWarps.Size(); ++wIdx) {
                evalWarpLanes( _FastWarps[wIdx], 0, _FastWarps[wIdx]._Count);
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
};

} // namespace trellis::rube

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_RUBE_ENGINE_H

