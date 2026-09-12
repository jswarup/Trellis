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
// GPU Backend Stubs (RustGpu / SPIR-V and CudaOxide / PTX)

class RustGpuBuffer : public IComputeBuffer
{
    std::string _Label{};
public:
    explicit RustGpuBuffer( const char* label) : _Label( label ? label : "rustgpu_buffer") {}
    uint64_t Size( void) const override { return 0; }
    const char* Label( void) const override { return _Label.c_str(); }
    SwarmError Write( silo::Arr< const uint8_t>) override { return SwarmError::UnsupportedBackend( BackendKind::RustGpu); }
    silo::Buff< uint8_t> Read( void) const override { return silo::Buff< uint8_t>(); }
};

class RustGpuKernel : public IComputeKernel
{
    std::string _Name{};
public:
    explicit RustGpuKernel( const char* name) : _Name( name ? name : "rustgpu_kernel") {}
    const char* Name( void) const override { return _Name.c_str(); }
    BackendKind Backend( void) const override { return BackendKind::RustGpu; }
};

class RustGpuDevice : public IComputeDevice
{
public:
    BackendKind Backend( void) const override { return BackendKind::RustGpu; }
    std::unique_ptr< IComputeBuffer> CreateBuffer( const char* label, uint64_t, BufferUsage) override
    {
        return std::make_unique< RustGpuBuffer>( label);
    }
    std::unique_ptr< IComputeBuffer> CreateBufferInit( const char* label, silo::Arr< const uint8_t>, BufferUsage) override
    {
        return std::make_unique< RustGpuBuffer>( label);
    }
    std::unique_ptr< IComputeKernel> CompileKernel( const char* label, const char*, const KernelSource&) override
    {
        return std::make_unique< RustGpuKernel>( label);
    }
    SwarmError Dispatch( const IComputeKernel&, silo::Arr< IComputeBuffer*>, WorkgroupDim) override
    {
        return SwarmError::UnsupportedBackend( BackendKind::RustGpu);
    }
    SwarmError Synchronize( void) override
    {
        return SwarmError::UnsupportedBackend( BackendKind::RustGpu);
    }
};

class CudaOxideBuffer : public IComputeBuffer
{
    std::string _Label{};
public:
    explicit CudaOxideBuffer( const char* label) : _Label( label ? label : "cudaoxide_buffer") {}
    uint64_t Size( void) const override { return 0; }
    const char* Label( void) const override { return _Label.c_str(); }
    SwarmError Write( silo::Arr< const uint8_t>) override { return SwarmError::UnsupportedBackend( BackendKind::CudaOxide); }
    silo::Buff< uint8_t> Read( void) const override { return silo::Buff< uint8_t>(); }
};

class CudaOxideKernel : public IComputeKernel
{
    std::string _Name{};
public:
    explicit CudaOxideKernel( const char* name) : _Name( name ? name : "cudaoxide_kernel") {}
    const char* Name( void) const override { return _Name.c_str(); }
    BackendKind Backend( void) const override { return BackendKind::CudaOxide; }
};

class CudaOxideDevice : public IComputeDevice
{
public:
    BackendKind Backend( void) const override { return BackendKind::CudaOxide; }
    std::unique_ptr< IComputeBuffer> CreateBuffer( const char* label, uint64_t, BufferUsage) override
    {
        return std::make_unique< CudaOxideBuffer>( label);
    }
    std::unique_ptr< IComputeBuffer> CreateBufferInit( const char* label, silo::Arr< const uint8_t>, BufferUsage) override
    {
        return std::make_unique< CudaOxideBuffer>( label);
    }
    std::unique_ptr< IComputeKernel> CompileKernel( const char* label, const char*, const KernelSource&) override
    {
        return std::make_unique< CudaOxideKernel>( label);
    }
    SwarmError Dispatch( const IComputeKernel&, silo::Arr< IComputeBuffer*>, WorkgroupDim) override
    {
        return SwarmError::UnsupportedBackend( BackendKind::CudaOxide);
    }
    SwarmError Synchronize( void) override
    {
        return SwarmError::UnsupportedBackend( BackendKind::CudaOxide);
    }
};

//-------------------------------------------------------------------------------------------------
// SwarmEngine: unified high-level compute engine

class SwarmEngine
{
    std::unique_ptr< IComputeDevice> _Device{};

public:
    explicit SwarmEngine( BackendKind backend = BackendKind::Cpu)
    {
        switch ( backend) {
        case BackendKind::Cpu:
            _Device = std::make_unique< CpuDevice>();
            break;
        case BackendKind::RustGpu:
            _Device = std::make_unique< RustGpuDevice>();
            break;
        case BackendKind::CudaOxide:
            _Device = std::make_unique< CudaOxideDevice>();
            break;
        }
    }

    static SwarmEngine Auto( void)
    {
        // Prioritize CPU as the fully implemented backend in Trellis
        return SwarmEngine( BackendKind::Cpu);
    }

    IComputeDevice& Device( void) noexcept
    {
        return *_Device;
    }

    const IComputeDevice& Device( void) const noexcept
    {
        return *_Device;
    }

    BackendKind Backend( void) const noexcept
    {
        return _Device->Backend();
    }

    SwarmError ExecuteOp(
        StandardOp op,
        silo::Arr< IComputeBuffer*> buffers,
        WorkgroupDim dim)
    {
        const char* label = StandardOpLabel( op);
        const char* entryPoint = StandardOpEntryPoint( op, Backend());
        KernelSource source = StandardOpKernelSource( op, Backend());

        auto kernel = _Device->CompileKernel( label, entryPoint, source);
        if ( !kernel) {
            return SwarmError::CompilationError( "Failed to compile standard op kernel");
        }

        auto err = _Device->Dispatch( *kernel, buffers, dim);
        if ( err.IsError()) {
            return err;
        }

        return _Device->Synchronize();
    }
};

} // namespace trellis::swarm
