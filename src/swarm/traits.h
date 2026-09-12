#ifndef TRELLIS_SWARM_TRAITS_H
#define TRELLIS_SWARM_TRAITS_H

//-------------------------------------------------------------------------------------------------

#include "silo/arr.h"
#include "silo/buff.h"

#include <cstdint>
#include <functional>
#include <memory>
#include <string>
#include <string_view>

//-------------------------------------------------------------------------------------------------

namespace trellis::swarm {

//-------------------------------------------------------------------------------------------------
// Target hardware / runtime backend kind.

enum class BackendKind
{
    Cpu,
    RustGpu,
    CudaOxide,
};

inline const char* BackendKindToString( BackendKind kind) noexcept
{
    switch ( kind) {
    case BackendKind::Cpu:       return "CPU";
    case BackendKind::RustGpu:   return "Rust-GPU (WebGPU/SPIR-V)";
    case BackendKind::CudaOxide: return "Cuda-Oxide (CUDA/PTX)";
    default:                     return "Unknown";
    }
}

//-------------------------------------------------------------------------------------------------
// Buffer usage flags for host and device memory management.

struct BufferUsage
{
    uint32_t bits{0};

    static constexpr BufferUsage Storage( void)   noexcept { return BufferUsage{1u << 0}; }
    static constexpr BufferUsage Uniform( void)   noexcept { return BufferUsage{1u << 1}; }
    static constexpr BufferUsage ReadOnly( void)  noexcept { return BufferUsage{1u << 2}; }
    static constexpr BufferUsage ReadWrite( void) noexcept { return BufferUsage{1u << 3}; }
    static constexpr BufferUsage CopySrc( void)   noexcept { return BufferUsage{1u << 4}; }
    static constexpr BufferUsage CopyDst( void)   noexcept { return BufferUsage{1u << 5}; }

    constexpr bool Contains( BufferUsage other) const noexcept
    {
        return ( bits & other.bits) == other.bits;
    }

    constexpr BufferUsage operator|( BufferUsage other) const noexcept
    {
        return BufferUsage{bits | other.bits};
    }

    constexpr BufferUsage& operator|=( BufferUsage other) noexcept
    {
        bits |= other.bits;
        return *this;
    }

    constexpr bool operator==( const BufferUsage& other) const noexcept = default;
};

//-------------------------------------------------------------------------------------------------
// 3D workgroup / threadblock dispatch dimensions.

struct WorkgroupDim
{
    uint32_t _X{1};
    uint32_t _Y{1};
    uint32_t _Z{1};

    constexpr WorkgroupDim( void) noexcept = default;
    constexpr WorkgroupDim( uint32_t x, uint32_t y = 1, uint32_t z = 1) noexcept
        : _X{x}, _Y{y}, _Z{z}
    {
    }

    static constexpr WorkgroupDim Linear( uint32_t x) noexcept
    {
        return WorkgroupDim{x, 1, 1};
    }

    constexpr uint64_t Total( void) const noexcept
    {
        return static_cast< uint64_t>( _X) * _Y * _Z;
    }
};

//-------------------------------------------------------------------------------------------------
// Function signature for CPU SIMT kernel closures.
// Parameters: inputs, outputs, gidX, gidY, gidZ

using CpuKernelFn = std::function< void(
    silo::Arr< silo::Arr< const uint8_t>> inputs,
    silo::Arr< silo::Arr< uint8_t>> outputs,
    uint32_t gidX,
    uint32_t gidY,
    uint32_t gidZ
)>;

//-------------------------------------------------------------------------------------------------
// Unified compute kernel source representation.

enum class KernelSourceKind
{
    Wgsl,
    SpirV,
    Ptx,
    CpuClosure,
};

struct KernelSource
{
    KernelSourceKind                _Kind{KernelSourceKind::CpuClosure};
    std::string_view                _CodeStr{};
    silo::Arr< const uint8_t>       _ByteCode{};
    CpuKernelFn                     _Closure{};

    static KernelSource Wgsl( std::string_view code) noexcept
    {
        KernelSource s;
        s._Kind = KernelSourceKind::Wgsl;
        s._CodeStr = code;
        return s;
    }

    static KernelSource SpirV( silo::Arr< const uint8_t> bytes) noexcept
    {
        KernelSource s;
        s._Kind = KernelSourceKind::SpirV;
        s._ByteCode = bytes;
        return s;
    }

    static KernelSource Ptx( std::string_view code) noexcept
    {
        KernelSource s;
        s._Kind = KernelSourceKind::Ptx;
        s._CodeStr = code;
        return s;
    }

    static KernelSource Cpu( CpuKernelFn fn)
    {
        KernelSource s;
        s._Kind = KernelSourceKind::CpuClosure;
        s._Closure = std::move( fn);
        return s;
    }
};

//-------------------------------------------------------------------------------------------------
// Error types occurring during compute operations.

enum class SwarmErrorKind
{
    None,
    DeviceUnavailable,
    CompilationError,
    BufferError,
    ExecutionError,
    UnsupportedBackend,
    InvalidKernelSource,
};

struct SwarmError
{
    SwarmErrorKind  _Kind{SwarmErrorKind::None};
    std::string     _Message{};

    static SwarmError Ok( void) noexcept
    {
        return SwarmError{SwarmErrorKind::None, ""};
    }

    static SwarmError DeviceUnavailable( std::string msg)
    {
        return SwarmError{SwarmErrorKind::DeviceUnavailable, std::move( msg)};
    }

    static SwarmError CompilationError( std::string msg)
    {
        return SwarmError{SwarmErrorKind::CompilationError, std::move( msg)};
    }

    static SwarmError BufferError( std::string msg)
    {
        return SwarmError{SwarmErrorKind::BufferError, std::move( msg)};
    }

    static SwarmError ExecutionError( std::string msg)
    {
        return SwarmError{SwarmErrorKind::ExecutionError, std::move( msg)};
    }

    static SwarmError UnsupportedBackend( BackendKind backend)
    {
        return SwarmError{SwarmErrorKind::UnsupportedBackend, BackendKindToString( backend)};
    }

    static SwarmError InvalidKernelSource( std::string msg)
    {
        return SwarmError{SwarmErrorKind::InvalidKernelSource, std::move( msg)};
    }

    bool IsOk( void) const noexcept { return _Kind == SwarmErrorKind::None; }
    bool IsError( void) const noexcept { return _Kind != SwarmErrorKind::None; }
};

//-------------------------------------------------------------------------------------------------
// Common interface for host/device compute buffers.

class IComputeBuffer
{
public:
    virtual ~IComputeBuffer( void) = default;

    virtual uint64_t Size( void) const = 0;
    virtual const char* Label( void) const = 0;
    virtual SwarmError Write( silo::Arr< const uint8_t> data) = 0;
    virtual silo::Buff< uint8_t> Read( void) const = 0;
};

//-------------------------------------------------------------------------------------------------
// Common interface for compiled compute kernels.

class IComputeKernel
{
public:
    virtual ~IComputeKernel( void) = default;

    virtual const char* Name( void) const = 0;
    virtual BackendKind Backend( void) const = 0;
};

//-------------------------------------------------------------------------------------------------
// Common interface for compute devices (CPU, Rust-GPU, Cuda-Oxide).

class IComputeDevice
{
public:
    virtual ~IComputeDevice( void) = default;

    virtual BackendKind Backend( void) const = 0;

    virtual std::unique_ptr< IComputeBuffer> CreateBuffer(
        const char* label,
        uint64_t size,
        BufferUsage usage
    ) = 0;

    virtual std::unique_ptr< IComputeBuffer> CreateBufferInit(
        const char* label,
        silo::Arr< const uint8_t> data,
        BufferUsage usage
    ) = 0;

    virtual std::unique_ptr< IComputeKernel> CompileKernel(
        const char* label,
        const char* entryPoint,
        const KernelSource& source
    ) = 0;

    virtual SwarmError Dispatch(
        const IComputeKernel& kernel,
        silo::Arr< IComputeBuffer*> buffers,
        WorkgroupDim dim
    ) = 0;

    virtual SwarmError Synchronize( void) = 0;
};

} // namespace trellis::swarm

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SWARM_TRAITS_H

