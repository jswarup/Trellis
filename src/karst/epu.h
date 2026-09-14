// epu.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "karst/dchan.h"
#include "swarm/cpu.h"
#include "swarm/traits.h"

#include <cstdint>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstVPU — Edge Processing Unit for near-memory compute on DDR5 channels via Swarm kernels.

class KarstVPU
{
private:
    uint32_t                                _VPUIdx{0};
    swarm::ComputeDevice*                   _Device{nullptr};
    std::unique_ptr< swarm::ComputeKernel>  _Kernel{};
    uint32_t                                _Dispatches{0};

public:
    KarstVPU( void) = default;

    KarstVPU( swarm::ComputeDevice& device, uint32_t epuIdx)
        : _VPUIdx( epuIdx),
          _Device( &device),
          _Dispatches( 0)
    {
        _Kernel = swarm::ComputeDevice::DoubleKernel();
    }

    uint32_t VPUIdx( void) const noexcept
    {
        return _VPUIdx;
    }

    uint32_t Dispatches( void) const noexcept
    {
        return _Dispatches;
    }

    swarm::SwarmError Dispatch( KarstDChan& chan, swarm::WorkgroupDim dim)
    {
        if ( !_Device || !_Kernel || !chan.Buffer()) {
            return swarm::SwarmError::ExecutionError( "EPU not initialized or channel buffer null");
        }

        swarm::ComputeBuffer* bufs[1] = {chan.Buffer()};
        silo::Arr< swarm::ComputeBuffer*> bufArr( bufs, 1);

        const swarm::SwarmError err = _Device->Dispatch( *_Kernel, bufArr, dim);
        if ( err.IsOk()) {
            _Dispatches++;
        }
        return err;
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

