// module.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/coro_kernel.h"
#include "rube/port.h"
#include "rube/trigger.h"
#include "silo/buff.h"
#include "silo/seg.h"

#include <cstdint>
#include <optional>
#include <string>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// Kernel operation types for fast gate/arithmetic modules.
// Full set matching Kosh rube/module.rs.

enum class KernelOp
{
    Nand,
    And,
    Or,
    Not,
    Xor,
    Nor,
    Xnor,
    Add,
    Sub,
    Shl,
    Shr,
};

template < typename T = uint64_t>
inline T EvalRaw( KernelOp op, T in1, T in2, uint64_t mask = ~0ULL) noexcept
{
    uint64_t res = 0;
    const uint64_t a = static_cast< uint64_t>( in1);
    const uint64_t b = static_cast< uint64_t>( in2);
    switch ( op) {
    case KernelOp::Nand: res = ~( a & b); break;
    case KernelOp::And:  res = a & b; break;
    case KernelOp::Or:   res = a | b; break;
    case KernelOp::Not:  res = ~a; break;
    case KernelOp::Xor:  res = a ^ b; break;
    case KernelOp::Nor:  res = ~( a | b); break;
    case KernelOp::Xnor: res = ~( a ^ b); break;
    case KernelOp::Add:  res = a + b; break;
    case KernelOp::Sub:  res = a - b; break;
    case KernelOp::Shl:  res = a << ( b & 63); break;
    case KernelOp::Shr:  res = a >> ( b & 63); break;
    }
    return static_cast< T>( res & mask);
}

template < typename T = uint64_t>
struct Eval4Result
{
    T    _Val{0};
    bool _IsX{false};
    bool _IsI{false};
};

template < typename T = uint64_t>
inline Eval4Result< T> Eval4State(
    KernelOp op,
    T in1, bool x1, bool i1,
    T in2, bool x2, bool i2,
    uint64_t mask = ~0ULL) noexcept
{
    Eval4Result< T> res;
    const uint64_t a = static_cast< uint64_t>( in1);
    const uint64_t b = static_cast< uint64_t>( in2);

    switch ( op) {
    case KernelOp::Nand:
    case KernelOp::And: {
        const bool isFalse1 = !x1 && !i1 && ( ( a & 1) == 0);
        const bool isFalse2 = !x2 && !i2 && ( ( b & 1) == 0);
        if ( isFalse1 || isFalse2) {
            res._Val = ( op == KernelOp::And) ? static_cast< T>( 0) : static_cast< T>( 1);
            res._IsX = false;
            res._IsI = false;
        } else if ( x1 || i1 || x2 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            const uint64_t andVal = a & b;
            res._Val = static_cast< T>( ( op == KernelOp::And) ? andVal : ~andVal);
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Nor:
    case KernelOp::Or: {
        const bool isTrue1 = !x1 && !i1 && ( ( a & 1) != 0);
        const bool isTrue2 = !x2 && !i2 && ( ( b & 1) != 0);
        if ( isTrue1 || isTrue2) {
            res._Val = ( op == KernelOp::Or) ? static_cast< T>( 1) : static_cast< T>( 0);
            res._IsX = false;
            res._IsI = false;
        } else if ( x1 || i1 || x2 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            const uint64_t orVal = a | b;
            res._Val = static_cast< T>( ( op == KernelOp::Or) ? orVal : ~orVal);
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Not: {
        if ( x1 || i1) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            if ( a == 1) {
                res._Val = static_cast< T>( 0);
            } else if ( a == 0) {
                res._Val = static_cast< T>( 1);
            } else {
                res._Val = static_cast< T>( ~a);
            }
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Xor: {
        if ( x1 || i1 || x2 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( a ^ b);
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Xnor: {
        if ( x1 || i1 || x2 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( ~( a ^ b));
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Add: {
        if ( x1 || x2 || i1 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( a + b);
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Sub: {
        if ( x1 || x2 || i1 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( a - b);
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Shl: {
        if ( x1 || x2 || i1 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( a << ( b & 63));
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    case KernelOp::Shr: {
        if ( x1 || x2 || i1 || i2) {
            res._Val = static_cast< T>( 0);
            res._IsX = true;
            res._IsI = false;
        } else {
            res._Val = static_cast< T>( a >> ( b & 63));
            res._IsX = false;
            res._IsI = false;
        }
        break;
    }
    }
    res._Val = static_cast< T>( static_cast< uint64_t>( res._Val) & mask);
    return res;
}

//-------------------------------------------------------------------------------------------------

enum class KernelKindType
{
    None,
    Fast,
    Coro,
};

struct KernelKind
{
    KernelKindType              _Type{KernelKindType::None};
    std::optional< KernelOp>    _Op{std::nullopt};
    CoroKernelFactory           _CoroFactory{nullptr};

    KernelKind( void) = default;

    KernelKind( KernelKindType type, std::optional< KernelOp> op) noexcept
        : _Type{type}, _Op{op}
    {
    }

    KernelKind( KernelKindType type, CoroKernelFactory factory)
        : _Type{type}, _CoroFactory{std::move( factory)}
    {
    }

    static KernelKind None( void) noexcept
    {
        return KernelKind{KernelKindType::None, std::nullopt};
    }

    static KernelKind Fast( KernelOp op) noexcept
    {
        return KernelKind{KernelKindType::Fast, op};
    }

    static KernelKind Coro( CoroKernelFactory factory)
    {
        return KernelKind{KernelKindType::Coro, std::move( factory)};
    }

    constexpr bool IsNone( void) const noexcept
    {
        return _Type == KernelKindType::None;
    }

    constexpr bool IsCoro( void) const noexcept
    {
        return _Type == KernelKindType::Coro;
    }

    constexpr std::optional< KernelOp> ToFastOp( void) const noexcept
    {
        return _Op;
    }

    const CoroKernelFactory& ToCoroFactory( void) const noexcept
    {
        return _CoroFactory;
    }

    std::pair< uint8_t, size_t> ClassKey( void) const noexcept
    {
        if ( _Type == KernelKindType::None) {
            return {2, 0};
        }
        if ( _Type == KernelKindType::Fast) {
            return {0, static_cast< size_t>( *_Op)};
        }
        if ( _Type == KernelKindType::Coro) {
            return {4, _CoroFactory.target_type().hash_code()};
        }
        return {2, 0};
    }
};

//-------------------------------------------------------------------------------------------------
// FastWarp — Structure-of-Arrays (SoA) SIMT Warp for homogeneous FastModule blocks.
// Field order and types match Kosh rube/module.rs.

struct FastWarp
{
    KernelOp                _Op{KernelOp::Nand};
    uint32_t                _ModStart{0};
    uint32_t                _Count{0};
    uint64_t                _Mask{1};
    silo::Buff< TriggerId>  _In1{};
    silo::Buff< TriggerId>  _In2{};
    silo::Buff< TriggerId>  _Out{};

    FastWarp( void) = default;

    FastWarp(
        KernelOp op,
        uint32_t modStart,
        uint32_t count,
        uint64_t mask,
        silo::Buff< TriggerId> in1,
        silo::Buff< TriggerId> in2,
        silo::Buff< TriggerId> out)
        : _Op( op),
          _ModStart( modStart),
          _Count( count),
          _Mask( mask),
          _In1( std::move( in1)),
          _In2( std::move( in2)),
          _Out( std::move( out))
    {
    }
};

//-------------------------------------------------------------------------------------------------
// Module — logical circuit block.

struct Module
{
    ModuleId        _Id{};
    ModuleId        _Parent{};
    std::string     _Name{};
    silo::USeg      _InPorts{0, 0};
    silo::USeg      _OutPorts{0, 0};
    silo::USeg      _SubModules{0, 0};
    silo::USeg      _Descendents{0, 0};
    KernelKind      _Kernel{KernelKind::None()};
    bool            _IsSealed{false};

    Module( void) = default;

    Module(
        ModuleId id,
        ModuleId parent,
        std::string name,
        silo::USeg inPorts,
        silo::USeg outPorts,
        KernelKind kernel)
        : _Id( id),
          _Parent( parent),
          _Name( std::move( name)),
          _InPorts( inPorts),
          _OutPorts( outPorts),
          _SubModules( 0, 0),
          _Descendents( 0, 0),
          _Kernel( kernel),
          _IsSealed( false)
    {
    }

    bool IsContainer( void) const noexcept
    {
        return _Kernel.IsNone();
    }
};

} // namespace trellis::rube
