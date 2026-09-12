// module.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/port.h"
#include "rube/reg.h"
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

inline uint64_t EvalRaw( KernelOp op, uint64_t in1, uint64_t in2, uint64_t mask) noexcept
{
    uint64_t res = 0;
    switch ( op) {
    case KernelOp::Nand: res = ~( in1 & in2); break;
    case KernelOp::And:  res = in1 & in2; break;
    case KernelOp::Or:   res = in1 | in2; break;
    case KernelOp::Not:  res = ~in1; break;
    case KernelOp::Xor:  res = in1 ^ in2; break;
    case KernelOp::Nor:  res = ~( in1 | in2); break;
    case KernelOp::Xnor: res = ~( in1 ^ in2); break;
    case KernelOp::Add:  res = in1 + in2; break;
    case KernelOp::Sub:  res = in1 - in2; break;
    case KernelOp::Shl:  res = in1 << ( in2 & 63); break;
    case KernelOp::Shr:  res = in1 >> ( in2 & 63); break;
    }
    return res & mask;
}

inline Reg Eval( KernelOp op, Reg in1, Reg in2, uint64_t mask) noexcept
{
    Reg res;
    switch ( op) {
    case KernelOp::Nand: res = ~( in1 & in2); break;
    case KernelOp::And:  res = in1 & in2; break;
    case KernelOp::Or:   res = in1 | in2; break;
    case KernelOp::Not:  res = ~in1; break;
    case KernelOp::Xor:  res = in1 ^ in2; break;
    case KernelOp::Nor:  res = ~( in1 | in2); break;
    case KernelOp::Xnor: res = ~( in1 ^ in2); break;
    case KernelOp::Add:
        if ( in1.IsX() || in2.IsX() || in1.IsI() || in2.IsI()) {
            res = Reg::Unknown();
        } else {
            res = Reg::Known( in1.Val() + in2.Val());
        }
        break;
    case KernelOp::Sub:
        if ( in1.IsX() || in2.IsX() || in1.IsI() || in2.IsI()) {
            res = Reg::Unknown();
        } else {
            res = Reg::Known( in1.Val() - in2.Val());
        }
        break;
    case KernelOp::Shl:
        if ( in1.IsX() || in2.IsX() || in1.IsI() || in2.IsI()) {
            res = Reg::Unknown();
        } else {
            res = Reg::Known( in1.Val() << ( in2.Val() & 63));
        }
        break;
    case KernelOp::Shr:
        if ( in1.IsX() || in2.IsX() || in1.IsI() || in2.IsI()) {
            res = Reg::Unknown();
        } else {
            res = Reg::Known( in1.Val() >> ( in2.Val() & 63));
        }
        break;
    }
    return res.Masked( mask);
}

//-------------------------------------------------------------------------------------------------

enum class KernelKindType
{
    None,
    Fast,
};

struct KernelKind
{
    KernelKindType          _Type{KernelKindType::None};
    std::optional< KernelOp> _Op{std::nullopt};

    static constexpr KernelKind None( void) noexcept
    {
        return KernelKind{KernelKindType::None, std::nullopt};
    }

    static constexpr KernelKind Fast( KernelOp op) noexcept
    {
        return KernelKind{KernelKindType::Fast, op};
    }

    constexpr bool IsNone( void) const noexcept
    {
        return _Type == KernelKindType::None;
    }

    constexpr std::optional< KernelOp> ToFastOp( void) const noexcept
    {
        return _Op;
    }

    constexpr std::pair< uint8_t, size_t> ClassKey( void) const noexcept
    {
        if ( _Type == KernelKindType::None) {
            return {2, 0};
        }
        return {0, static_cast< size_t>( *_Op)};
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
