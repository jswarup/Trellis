// engine.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "swarm/cpu.h"
#include "swarm/ops.h"
#include "swarm/traits.h"

#include <memory>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::swarm {

//-------------------------------------------------------------------------------------------------
// SwarmEngine: unified high-level compute engine

class SwarmEngine
{
    ComputeDevice _Device{};

public:
    explicit SwarmEngine( BackendKind backend = BackendKind::Cpu)
        : _Device( backend)
    {
    }

    static SwarmEngine Auto( void)
    {
        // Prioritize CPU as the fully implemented backend in Trellis
        return SwarmEngine( BackendKind::Cpu);
    }

    ComputeDevice& Device( void) noexcept
    {
        return _Device;
    }

    const ComputeDevice& Device( void) const noexcept
    {
        return _Device;
    }

    BackendKind Backend( void) const noexcept
    {
        return _Device.Backend();
    }

    SwarmError ExecuteOp(
        StandardOp op,
        silo::Arr< ComputeBuffer*> buffers,
        WorkgroupDim dim)
    {
        const char* label = StandardOpLabel( op);
        const char* entryPoint = StandardOpEntryPoint( op, Backend());
        KernelSource source = StandardOpKernelSource( op, Backend());

        auto kernel = _Device.CompileKernel( label, entryPoint, source);
        if ( !kernel) {
            return SwarmError::CompilationError( "Failed to compile standard op kernel");
        }

        auto err = _Device.Dispatch( *kernel, buffers, dim);
        if ( err.IsError()) {
            return err;
        }

        return _Device.Synchronize();
    }
};

} // namespace trellis::swarm
