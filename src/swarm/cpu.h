// cpu.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "heist/atelier.h"
#include "silo/buff.h"
#include "silo/stash.h"
#include "stalks/atm.h"
#include "swarm/ops.h"
#include "swarm/traits.h"

#include <algorithm>
#include <cstring>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::swarm {

//-------------------------------------------------------------------------------------------------
// ComputeDevice executing SIMT compute kernels over host CPU worker threads (or returning
// UnsupportedBackend for unconfigured hardware backends).

class ComputeDevice
{
    BackendKind _Backend{BackendKind::Cpu};
    uint32_t    _WorkerCount{4};

public:
    ComputeDevice( void)
        : _Backend( BackendKind::Cpu),
          _WorkerCount( heist::Atelier::DefaultThreadCount())
    {
    }

    explicit ComputeDevice( uint32_t workers)
        : _Backend( BackendKind::Cpu),
          _WorkerCount( std::max( 1u, workers))
    {
    }

    explicit ComputeDevice( BackendKind backend, uint32_t workers = 0)
        : _Backend( backend),
          _WorkerCount( workers > 0 ? workers : heist::Atelier::DefaultThreadCount())
    {
    }

    BackendKind Backend( void) const noexcept
    {
        return _Backend;
    }

    uint32_t WorkerCount( void) const noexcept
    {
        return _WorkerCount;
    }

    static std::unique_ptr< ComputeKernel> DoubleKernel( void)
    {
        return std::make_unique< ComputeKernel>(
            StandardOpLabel( StandardOp::Double),
            "main",
            BackendKind::Cpu,
            StandardOpCpuKernelFn( StandardOp::Double)
        );
    }

    static std::unique_ptr< ComputeKernel> VectorAddKernel( void)
    {
        return std::make_unique< ComputeKernel>(
            StandardOpLabel( StandardOp::VectorAdd),
            "main",
            BackendKind::Cpu,
            StandardOpCpuKernelFn( StandardOp::VectorAdd)
        );
    }

    static std::unique_ptr< ComputeKernel> CollatzKernel( void)
    {
        return std::make_unique< ComputeKernel>(
            StandardOpLabel( StandardOp::Collatz),
            "main",
            BackendKind::Cpu,
            StandardOpCpuKernelFn( StandardOp::Collatz)
        );
    }

    static std::unique_ptr< ComputeKernel> PointCloudKernel( void)
    {
        return std::make_unique< ComputeKernel>(
            StandardOpLabel( StandardOp::PointCloud),
            "pts_pointcloud_cs",
            BackendKind::Cpu,
            StandardOpCpuKernelFn( StandardOp::PointCloud)
        );
    }

    static std::unique_ptr< ComputeKernel> CameraTransformKernel( void)
    {
        return std::make_unique< ComputeKernel>(
            StandardOpLabel( StandardOp::CameraTransform),
            "camera_transform_cs",
            BackendKind::Cpu,
            StandardOpCpuKernelFn( StandardOp::CameraTransform)
        );
    }

    std::unique_ptr< ComputeBuffer> CreateBuffer(
        const char* label,
        uint64_t size,
        BufferUsage usage)
    {
        return std::make_unique< ComputeBuffer>( label, size, usage, _Backend);
    }

    std::unique_ptr< ComputeBuffer> CreateBufferInit(
        const char* label,
        silo::Arr< const uint8_t> data,
        BufferUsage usage)
    {
        return std::make_unique< ComputeBuffer>( label, data, usage, _Backend);
    }

    std::unique_ptr< ComputeKernel> CompileKernel(
        const char* label,
        const char* entryPoint,
        const KernelSource& source)
    {
        if ( _Backend != BackendKind::Cpu) {
            return std::make_unique< ComputeKernel>( label ? label : "kernel", entryPoint ? entryPoint : "main", _Backend, nullptr);
        }

        switch ( source._Kind) {
        case KernelSourceKind::CpuClosure:
            return std::make_unique< ComputeKernel>( label ? label : "cpu_kernel", entryPoint ? entryPoint : "main", _Backend, source._Closure);
        case KernelSourceKind::Wgsl: {
            std::string_view src = source._CodeStr;
            std::string_view ep = entryPoint ? entryPoint : "";
            if ( src.find( "pts_pointcloud") != std::string_view::npos || ep.find( "pts_pointcloud") != std::string_view::npos) {
                return PointCloudKernel();
            } else if ( src.find( "collatz") != std::string_view::npos || ep.find( "collatz") != std::string_view::npos) {
                return CollatzKernel();
            } else if ( src.find( "vecadd") != std::string_view::npos || ep.find( "vecadd") != std::string_view::npos) {
                return VectorAddKernel();
            } else if ( src.find( "double") != std::string_view::npos || ep.find( "double") != std::string_view::npos) {
                return DoubleKernel();
            } else if ( src.find( "camera_transform") != std::string_view::npos || ep.find( "camera_transform") != std::string_view::npos) {
                return CameraTransformKernel();
            }
            return DoubleKernel();
        }
        case KernelSourceKind::SpirV: {
            std::string_view ep = entryPoint ? entryPoint : "";
            std::string_view lbl = label ? label : "";
            if ( ep == "pts_pointcloud_cs" || lbl.find( "pointcloud") != std::string_view::npos) {
                return PointCloudKernel();
            } else if ( ep == "camera_transform_cs" || lbl.find( "camera_transform") != std::string_view::npos) {
                return CameraTransformKernel();
            } else if ( ep == "collatz_cs" || lbl.find( "collatz") != std::string_view::npos) {
                return CollatzKernel();
            }
            return DoubleKernel();
        }
        case KernelSourceKind::Ptx: {
            std::string_view src = source._CodeStr;
            std::string_view ep = entryPoint ? entryPoint : "";
            if ( src.find( "pointcloud") != std::string_view::npos || ep.find( "pointcloud") != std::string_view::npos) {
                return PointCloudKernel();
            } else if ( src.find( "collatz") != std::string_view::npos || ep.find( "collatz") != std::string_view::npos) {
                return CollatzKernel();
            } else if ( src.find( "vecadd") != std::string_view::npos || ep.find( "vecadd") != std::string_view::npos) {
                return VectorAddKernel();
            } else if ( src.find( "camera_transform") != std::string_view::npos || ep.find( "camera_transform") != std::string_view::npos) {
                return CameraTransformKernel();
            }
            return DoubleKernel();
        }
        default:
            return DoubleKernel();
        }
    }

    SwarmError Dispatch(
        const ComputeKernel& kernel,
        silo::Arr< ComputeBuffer*> buffers,
        WorkgroupDim dim)
    {
        if ( _Backend != BackendKind::Cpu) {
            return SwarmError::UnsupportedBackend( _Backend);
        }

        if ( buffers.Size() == 0) {
            return SwarmError::Ok();
        }

        const uint32_t threadsX = dim._X * 64;
        const uint32_t threadsY = dim._Y;
        const uint32_t threadsZ = dim._Z;

        // Read all buffers into CPU memory
        silo::Buff< silo::Buff< uint8_t>> rawBuffers( buffers.Size(), [&]( uint32_t i) {
            return buffers[i]->Read();
        });

        // Determine input and output buffers
        // If 1 buffer: treat as read-write (input & output)
        // If >= 2 buffers: inputs are 0..N-2, output is N-1
        const uint32_t inCount = ( rawBuffers.Size() == 1) ? 1 : ( rawBuffers.Size() - 1);
        const uint32_t outCount = 1;

        silo::Buff< silo::Arr< const uint8_t>> inSlices( inCount, [&]( uint32_t i) {
            return rawBuffers[i].AsArr();
        });

        const uint32_t outIdx = rawBuffers.Size() - 1;
        silo::Buff< silo::Arr< uint8_t>> outSlices( outCount, [&]( uint32_t) {
            return rawBuffers[outIdx].AsArr();
        });

        auto& atelier = heist::Atelier::Instance();
        if ( !atelier.IsImmediate() && atelier.SzThreads() > 1) {
            heist::Maestro* mainMaestro = atelier.MainMaestro();
            const uint32_t chunkSize = 64;
            const uint32_t numChunks = ( threadsX + chunkSize - 1) / chunkSize;

            for ( uint32_t c = 0; c < numChunks; ++c) {
                const uint32_t startX = c * chunkSize;
                const uint32_t endX = std::min( startX + chunkSize, threadsX);

                mainMaestro->PostJob( stalks::WorkPtr::FromLambda(
                    [&kernel, inSlices, outSlices, startX, endX, threadsY, threadsZ]( stalks::IWorker*) {
                        for ( uint32_t z = 0; z < threadsZ; ++z) {
                            for ( uint32_t y = 0; y < threadsY; ++y) {
                                for ( uint32_t x = startX; x < endX; ++x) {
                                    kernel.Execute( inSlices.AsArr(), outSlices.AsArr(), x, y, z);
                                }
                            }
                        }
                    }
                ));
            }
            atelier.DoLaunch();
        } else {
            for ( uint32_t z = 0; z < threadsZ; ++z) {
                for ( uint32_t y = 0; y < threadsY; ++y) {
                    for ( uint32_t x = 0; x < threadsX; ++x) {
                        kernel.Execute( inSlices.AsArr(), outSlices.AsArr(), x, y, z);
                    }
                }
            }
        }

        // Write modified output buffer back to target buffer
        buffers[outIdx]->Write( rawBuffers[outIdx].AsArr());

        return SwarmError::Ok();
    }

    SwarmError Synchronize( void)
    {
        if ( _Backend != BackendKind::Cpu) {
            return SwarmError::UnsupportedBackend( _Backend);
        }
        return SwarmError::Ok();
    }
};

using CpuDevice = ComputeDevice;
using IComputeDevice = ComputeDevice;

} // namespace trellis::swarm
