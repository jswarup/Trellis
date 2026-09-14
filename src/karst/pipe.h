// pipe.h ---------------------------------------------------------------------------------------------------------
#pragma once

#include "karst/config.h"
#include "rube/rube.h"
#include "silo/arr.h"
#include "silo/fifo.h"

#include <cstdint>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstPipe — retiming pipeline module implementing mpipe pipeline_axi_channel FIFO semantics.
// Statically staged to model deterministic cycle-accurate delay and backpressure over long routes.

class KarstPipe
{
private:
    rube::ModuleId      _Id{};
    rube::PortId        _InValid{};
    rube::PortId        _InData{};
    rube::PortId        _DownReady{};
    rube::PortId        _UpReady{};
    rube::PortId        _OutValid{};
    rube::PortId        _OutData{};

public:
    KarstPipe( void) = default;

    KarstPipe(
        rube::Layout& layout,
        const char* name,
        uint32_t depth = k_LinkDepth,
        rube::ModuleId parent = rube::ModuleId{})
    {
        rube::PortDesc inDescs[3] = {
            rube::PortDesc( "InValid",   rube::PortType::Bool()),
            rube::PortDesc( "InData",    rube::PortType::U64Val()),
            rube::PortDesc( "DownReady", rube::PortType::Bool())
        };

        rube::PortDesc outDescs[3] = {
            rube::PortDesc( "UpReady",   rube::PortType::Bool()),
            rube::PortDesc( "OutValid",  rube::PortType::Bool()),
            rube::PortDesc( "OutData",   rube::PortType::U64Val())
        };

        const uint32_t maxDepth = ( depth == 0) ? 1 : depth;

        _Id = layout.AddCoroModule(
            name,
            parent,
            silo::Arr< const rube::PortDesc>( inDescs, 3),
            silo::Arr< const rube::PortDesc>( outDescs, 3),
            [maxDepth]() -> rube::CoroTask {
                silo::Fifo< uint64_t, k_LinkDepth> fifo;
                bool wasPresented = false;

                rube::CoroPorts in = co_await rube::CoroIn{};

                while ( true)
                {
                    if ( wasPresented && in.Get< bool>( 2) && !fifo.IsEmpty()) {
                        fifo.PopFront();
                    }

                    if ( in.Get< bool>( 0) && !fifo.IsFull()) {
                        fifo.PushBack( in[1]);
                    }

                    bool outValid = false;
                    uint64_t outData = 0;
                    if ( !fifo.IsEmpty()) {
                        outValid = true;
                        outData = fifo.Front();
                    }
                    wasPresented = outValid;

                    const bool upReady = !fifo.IsFull();

                    rube::CoroPorts out;
                    out.Push( upReady);
                    out.Push( outValid);
                    out.Push( outData);

                    in = co_yield out;
                }
            }
        );

        _InValid   = layout.InPort( _Id, 0);
        _InData    = layout.InPort( _Id, 1);
        _DownReady = layout.InPort( _Id, 2);

        _UpReady   = layout.OutPort( _Id, 0);
        _OutValid  = layout.OutPort( _Id, 1);
        _OutData   = layout.OutPort( _Id, 2);
    }

    constexpr rube::ModuleId Id( void) const noexcept { return _Id; }
    constexpr rube::PortId InValid( void) const noexcept { return _InValid; }
    constexpr rube::PortId InData( void) const noexcept { return _InData; }
    constexpr rube::PortId DownReady( void) const noexcept { return _DownReady; }
    constexpr rube::PortId UpReady( void) const noexcept { return _UpReady; }
    constexpr rube::PortId OutValid( void) const noexcept { return _OutValid; }
    constexpr rube::PortId OutData( void) const noexcept { return _OutData; }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

