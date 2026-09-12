#ifndef TRELLIS_SILO_DSET_H
#define TRELLIS_SILO_DSET_H

//-------------------------------------------------------------------------------------------------

#include "silo/stash.h"
#include <cassert>
#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// DisjointSet — union-find with path compression and union-by-rank.
// Modeled directly from Kosh silo/disjoint_set.rs.

class DisjointSet
{
public:
    Stash< uint32_t>    _Parent{};
    Stash< uint8_t>     _Rank{};

public:
    constexpr DisjointSet( void) noexcept = default;

    explicit DisjointSet( uint32_t capacity)
        : _Parent( capacity, 0, 0u),
          _Rank( capacity, 0, static_cast< uint8_t>( 0))
    {
    }

    uint32_t Size( void) const noexcept
    {
        return _Parent.Size();
    }

    uint32_t FindConst( uint32_t elem) const
    {
        assert( elem < Size() && "DisjointSet element out of bounds");
        uint32_t        curr = elem;
        while ( _Parent[curr] != curr) {
            curr = _Parent[curr];
        }
        return curr;
    }

    uint32_t Find( uint32_t elem)
    {
        assert( elem < Size() && "DisjointSet element out of bounds");
        uint32_t        curr = elem;
        while ( _Parent[curr] != curr) {
            curr = _Parent[curr];
        }
        const uint32_t  root = curr;

        uint32_t        node = elem;
        while ( _Parent[node] != node) {
            const uint32_t      next = _Parent[node];
            _Parent[node] = root;
            node = next;
        }
        return root;
    }

    uint32_t Union( uint32_t a, uint32_t b)
    {
        const uint32_t  rootA = Find( a);
        const uint32_t  rootB = Find( b);
        if ( rootA == rootB)
            return rootA;

        const uint8_t   rankA = _Rank[rootA];
        const uint8_t   rankB = _Rank[rootB];

        if ( rankA < rankB) {
            _Parent[rootA] = rootB;
            return rootB;
        } else if ( rankA > rankB) {
            _Parent[rootB] = rootA;
            return rootA;
        } else {
            _Parent[rootB] = rootA;
            _Rank[rootA] += 1;
            return rootA;
        }
    }

    bool Same( uint32_t a, uint32_t b)
    {
        return Find( a) == Find( b);
    }

    void Grow( uint32_t count)
    {
        const uint32_t  start = Size();
        for ( uint32_t i = 0; i < count; ++i) {
            const uint32_t      idx = start + i;
            _Parent.PushBack( idx);
            _Rank.PushBack( static_cast< uint8_t>( 0));
        }
    }

    void Clear( void)
    {
        _Parent.Clear();
        _Rank.Clear();
    }
};

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_DSET_H

