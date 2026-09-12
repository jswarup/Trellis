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
// In-memory compute buffer for CPU SIMT execution.

class CpuBuffer : public IComputeBuffer
{
    std::string                 _Label{};
    silo::Buff< uint8_t>        _Data{};
    mutable stalks::Spinlock    _Lock{};
    BufferUsage                 _Usage{};

public:
    CpuBuffer( const char* label, uint64_t size, BufferUsage usage)
        : _Label( label ? label : "cpu_buffer"),
          _Data( static_cast< uint32_t>( size), static_cast< uint8_t>( 0)),
          _Usage( usage)
    {
    }

    CpuBuffer( const char* label, silo::Arr< const uint8_t> data, BufferUsage usage)
        : _Label( label ? label : "cpu_buffer"),
          _Data( data.Size(), []( uint32_t) { return static_cast< uint8_t>( 0); }),
          _Usage( usage)
    {
        std::memcpy( _Data.Data(), data.Data(), data.Size());
    }

    uint64_t Size( void) const override
    {
        auto guard = _Lock.Lock();
        return _Data.Size();
    }

    const char* Label( void) const override
    {
        return _Label.c_str();
    }

    BufferUsage Usage( void) const noexcept
    {
        return _Usage;
    }

    SwarmError Write( silo::Arr< const uint8_t> data) override
    {
        auto guard = _Lock.Lock();
        if ( data.Size() > _Data.Size()) {
            _Data.Resize( data.Size(), []( uint32_t) { return static_cast< uint8_t>( 0); });
        }
        std::memcpy( _Data.Data(), data.Data(), data.Size());
        return SwarmError::Ok();
    }

    silo::Buff< uint8_t> Read( void) const override
    {
        auto guard = _Lock.Lock();
        silo::Buff< uint8_t> copy( _Data.Size(), []( uint32_t) { return static_cast< uint8_t>( 0); });
        if ( _Data.Size() > 0) {
            std::memcpy( copy.Data(), _Data.Data(), _Data.Size());
        }
        return copy;
    }
};

//-------------------------------------------------------------------------------------------------
// Executable kernel closure on the CPU.

class CpuKernel : public IComputeKernel
{
    std::string     _Name{};
    std::string     _EntryPoint{};
    CpuKernelFn     _KernelFn{};

public:
    CpuKernel( std::string name, std::string entryPoint, CpuKernelFn kernelFn)
        : _Name( std::move( name)),
          _EntryPoint( std::move( entryPoint)),
          _KernelFn( std::move( kernelFn))
    {
    }

    const char* Name( void) const override
    {
        return _Name.c_str();
    }

    BackendKind Backend( void) const override
    {
        return BackendKind::Cpu;
    }

    const char* EntryPoint( void) const noexcept
    {
        return _EntryPoint.c_str();
    }

    void Execute(
        silo::Arr< silo::Arr< const uint8_t>> inputs,
        silo::Arr< silo::Arr< uint8_t>> outputs,
        uint32_t gidX,
        uint32_t gidY,
        uint32_t gidZ) const
    {
        if ( _KernelFn) {
            _KernelFn( inputs, outputs, gidX, gidY, gidZ);
        }
    }
};

//-------------------------------------------------------------------------------------------------
// CPU compute device executing kernels over host CPU worker threads.

class CpuDevice : public IComputeDevice
{
    uint32_t    _WorkerCount{4};

public:
    CpuDevice( void)
        : _WorkerCount( heist::Atelier::DefaultThreadCount())
    {
    }

    explicit CpuDevice( uint32_t workers)
        : _WorkerCount( std::max( 1u, workers))
    {
    }

    BackendKind Backend( void) const override
    {
        return BackendKind::Cpu;
    }

    uint32_t WorkerCount( void) const noexcept
    {
        return _WorkerCount;
    }

    static std::unique_ptr< CpuKernel> DoubleKernel( void)
    {
        return std::make_unique< CpuKernel>(
            StandardOpLabel( StandardOp::Double),
            "main",
            StandardOpCpuKernelFn( StandardOp::Double)
        );
    }

    static std::unique_ptr< CpuKernel> VectorAddKernel( void)
    {
        return std::make_unique< CpuKernel>(
            StandardOpLabel( StandardOp::VectorAdd),
            "main",
            StandardOpCpuKernelFn( StandardOp::VectorAdd)
        );
    }

    static std::unique_ptr< CpuKernel> CollatzKernel( void)
    {
        return std::make_unique< CpuKernel>(
            StandardOpLabel( StandardOp::Collatz),
            "main",
            StandardOpCpuKernelFn( StandardOp::Collatz)
        );
    }

    static std::unique_ptr< CpuKernel> PointCloudKernel( void)
    {
        return std::make_unique< CpuKernel>(
            StandardOpLabel( StandardOp::PointCloud),
            "pts_pointcloud_cs",
            StandardOpCpuKernelFn( StandardOp::PointCloud)
        );
    }

    static std::unique_ptr< CpuKernel> CameraTransformKernel( void)
    {
        return std::make_unique< CpuKernel>(
            StandardOpLabel( StandardOp::CameraTransform),
            "camera_transform_cs",
            StandardOpCpuKernelFn( StandardOp::CameraTransform)
        );
    }

    std::unique_ptr< IComputeBuffer> CreateBuffer(
        const char* label,
        uint64_t size,
        BufferUsage usage) override
    {
        return std::make_unique< CpuBuffer>( label, size, usage);
    }

    std::unique_ptr< IComputeBuffer> CreateBufferInit(
        const char* label,
        silo::Arr< const uint8_t> data,
        BufferUsage usage) override
    {
        return std::make_unique< CpuBuffer>( label, data, usage);
    }

    std::unique_ptr< IComputeKernel> CompileKernel(
        const char* label,
        const char* entryPoint,
        const KernelSource& source) override
    {
        switch ( source._Kind) {
        case KernelSourceKind::CpuClosure:
            return std::make_unique< CpuKernel>( label ? label : "cpu_kernel", entryPoint ? entryPoint : "main", source._Closure);
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
        const IComputeKernel& kernel,
        silo::Arr< IComputeBuffer*> buffers,
        WorkgroupDim dim) override
    {
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

        const CpuKernel* cpuK = dynamic_cast< const CpuKernel*>( &kernel);

        auto& atelier = heist::Atelier::Instance();
        if ( !atelier.IsImmediate() && atelier.SzThreads() > 1) {
            heist::Maestro* mainMaestro = atelier.MainMaestro();
            const uint32_t chunkSize = 64;
            const uint32_t numChunks = ( threadsX + chunkSize - 1) / chunkSize;

            for ( uint32_t c = 0; c < numChunks; ++c) {
                const uint32_t startX = c * chunkSize;
                const uint32_t endX = std::min( startX + chunkSize, threadsX);

                mainMaestro->PostJob( stalks::WorkPtr::FromLambda(
                    [cpuK, inSlices, outSlices, startX, endX, threadsY, threadsZ]( stalks::IWorker*) {
                        for ( uint32_t z = 0; z < threadsZ; ++z) {
                            for ( uint32_t y = 0; y < threadsY; ++y) {
                                for ( uint32_t x = startX; x < endX; ++x) {
                                    if ( cpuK) {
                                        cpuK->Execute( inSlices.AsArr(), outSlices.AsArr(), x, y, z);
                                    }
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
                        if ( cpuK) {
                            cpuK->Execute( inSlices.AsArr(), outSlices.AsArr(), x, y, z);
                        }
                    }
                }
            }
        }

        // Write modified output buffer back to target buffer
        buffers[outIdx]->Write( rawBuffers[outIdx].AsArr());

        return SwarmError::Ok();
    }

    SwarmError Synchronize( void) override
    {
        return SwarmError::Ok();
    }
};

} // namespace trellis::swarm
