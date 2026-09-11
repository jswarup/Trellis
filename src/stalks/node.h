#ifndef TRELLIS_STALKS_NODE_H
#define TRELLIS_STALKS_NODE_H

//-------------------------------------------------------------------------------------------------

#include <cstdint>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::stalks {

//-------------------------------------------------------------------------------------------------
// BinOp — binary operation types for abstract syntax and dependency graph composition.

enum class BinOp : uint64_t
{
    Sum  = 0,
    Prod = 1,
    Sub  = 2,
    Div  = 3,
    Pow  = 4,
    None = 5,
    Less = 6,
    Bor  = 7,
};

//-------------------------------------------------------------------------------------------------
// BinNode — generic binary composite tree node.

template < typename L, typename R, typename Op = BinOp>
struct BinNode
{
    L                   _Left;
    R                   _Right;
    Op                  _Op{BinOp::None};

    constexpr BinNode( L left, R right, Op op) noexcept
        : _Left( std::move( left)),
          _Right( std::move( right)),
          _Op( op)
    {
    }

    constexpr bool operator==( const BinNode&) const = default;
};

//-------------------------------------------------------------------------------------------------
// UniNode — generic unary tree node.

template < typename C, typename Op = BinOp>
struct UniNode
{
    C                   _Child;
    Op                  _Op{BinOp::None};

    constexpr UniNode( C child, Op op) noexcept
        : _Child( std::move( child)),
          _Op( op)
    {
    }

    constexpr bool operator==( const UniNode&) const = default;
};

//-------------------------------------------------------------------------------------------------
// Concept: CNode — marker concept for syntax tree elements.

template < typename T>
concept CNode = true;

} // namespace trellis::stalks

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_STALKS_NODE_H

