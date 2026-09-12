#ifndef TRELLIS_SILO_SEG_H
#define TRELLIS_SILO_SEG_H

//-------------------------------------------------------------------------------------------------

#include "stalks/work.h"

#include <compare>
#include <cstdint>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Seg — integer segment range representation modeled directly from Kosh useg.rs.
// Internally represents the closed range [_First, _Last].

template < typename TSzType = uint32_t>
struct Seg
{
    TSzType             _First{0};
    TSzType             _Last{0};

    struct SearchResult
    {
        bool            _Found{false};
        TSzType         _Index{0};

        constexpr explicit operator bool( void) const noexcept
        {
            return _Found;
        }
    };

    //---------------------------------------------------------------------------------------------
    // Constructors & Factories

    constexpr Seg( void) noexcept = default;

    constexpr Seg( TSzType first, TSzType sz) noexcept
        : _First( first),
          _Last( static_cast< TSzType>( first + sz - 1))
    {
    }

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries

    constexpr TSzType First( void) const noexcept
    {
        return _First;
    }

    constexpr TSzType Begin( void) const noexcept
    {
        return _First;
    }

    constexpr TSzType Last( void) const noexcept
    {
        return _Last;
    }

    constexpr TSzType End( void) const noexcept
    {
        return IsEmpty() ? _First : static_cast< TSzType>( _Last + 1);
    }

    constexpr TSzType Mid( void) const noexcept
    {
        if ( _Last >= _First)
            return _First + ( _Last - _First) / 2;

        return _First;
    }

    constexpr TSzType Size( void) const noexcept
    {
        if ( _Last >= _First)
            return static_cast< TSzType>( _Last + 1 - _First);

        return 0;
    }

    constexpr bool IsEmpty( void) const noexcept
    {
        return Size() == 0;
    }

    constexpr bool IsWithin( TSzType val) const noexcept
    {
        return ( val >= _First) && ( val <= _Last);
    }

    //---------------------------------------------------------------------------------------------
    // Slicing

    constexpr Seg LSnip( TSzType count) const noexcept
    {
        const TSzType   sz = Size();
        if ( sz <= count)
            return Seg( ~static_cast< TSzType>( 0), 0);

        return Seg( static_cast< TSzType>( _First + count), static_cast< TSzType>( sz - count));
    }

    constexpr Seg RSnip( TSzType count) const noexcept
    {
        const TSzType   sz = Size();
        if ( sz <= count)
            return Seg( ~static_cast< TSzType>( 0), 0);

        return Seg( _First, static_cast< TSzType>( sz - count));
    }

    //---------------------------------------------------------------------------------------------
    // Functional Operations

template < typename F>
    constexpr bool Span( F&& lambda) const
    {
        if ( IsEmpty())
            return true;

        for ( TSzType i = _First; i <= _Last; ++i) {
            if ( !lambda( i))
                return false;
        }
        return true;
    }

template < typename F>
    constexpr void Traverse( F&& lambda) const
    {
        if ( IsEmpty())
            return;

        for ( TSzType i = _First; i <= _Last; ++i)
            lambda( i);
    }

template < typename F>
    constexpr void TraverseRev( F&& lambda) const
    {
        if ( IsEmpty())
            return;

        for ( TSzType i = _Last + 1; i > _First; --i)
            lambda( static_cast< TSzType>( i - 1));
    }

    //---------------------------------------------------------------------------------------------
    // Sorting (Quicksort & Partition)

template < typename LessAt, typename SwapAt>
    TSzType Partition( LessAt&& lessAt, SwapAt&& swapAt) const
    {
        TSzType         mid = Mid();
        if ( lessAt( _First, mid))
            swapAt( _First, mid);

        TSzType         pivot = _First;
        LSnip( 1).Traverse( [&]( TSzType i) {
            if ( lessAt( i, _First)) {
                pivot = static_cast< TSzType>( pivot + 1);
                swapAt( pivot, i);
            }
        });

        if ( lessAt( pivot, _First))
            swapAt( _First, pivot);

        return pivot;
    }

template < typename LessAt, typename SwapAt>
    void QSort( LessAt&& lessAt, SwapAt&& swapAt) const
    {
        Seg             currentSeg = *this;
        while ( currentSeg.Size() > 1) {
            TSzType     pivot = currentSeg.Partition( lessAt, swapAt);
            Seg         useg1 = Seg( currentSeg._First, static_cast< TSzType>( pivot - currentSeg._First));
            Seg         useg2 = Seg( static_cast< TSzType>( pivot + 1), static_cast< TSzType>( currentSeg._Last - pivot));

            if ( useg1.Size() < useg2.Size()) {
                if ( useg1.Size() > 1)
                    useg1.QSort( lessAt, swapAt);
                currentSeg = useg2;
            } else {
                if ( useg2.Size() > 1)
                    useg2.QSort( lessAt, swapAt);
                currentSeg = useg1;
            }
        }
    }

template < typename LessAt, typename SwapAt>
    void DoQSort( stalks::IWorker* worker, LessAt lessAt, SwapAt swapAt) const
    {
        if ( worker == nullptr) {
            QSort( lessAt, swapAt);
            return;
        }

        Seg             currentSeg = *this;
        while ( currentSeg.Size() > 1) {
            if ( currentSeg.Size() < static_cast< TSzType>( 32)) {
                currentSeg.QSort( lessAt, swapAt);
                return;
            }

            TSzType     pivot = currentSeg.Partition( lessAt, swapAt);
            Seg         useg1 = Seg( currentSeg._First, static_cast< TSzType>( pivot - currentSeg._First));
            Seg         useg2 = Seg( static_cast< TSzType>( pivot + 1), static_cast< TSzType>( currentSeg._Last - pivot));

            if ( useg1.Size() > useg2.Size()) {
                if ( useg1.Size() > 1) {
                    worker->Post( [useg1, lessAt, swapAt]( stalks::IWorker* w) {
                        useg1.DoQSort( w, lessAt, swapAt);
                    });
                }
                currentSeg = useg2;
            } else {
                if ( useg2.Size() > 1) {
                    worker->Post( [useg2, lessAt, swapAt]( stalks::IWorker* w) {
                        useg2.DoQSort( w, lessAt, swapAt);
                    });
                }
                currentSeg = useg1;
            }
        }
    }

template < typename LessAt, typename SwapAt>
    void DoQSort( stalks::IWorker& worker, LessAt lessAt, SwapAt swapAt) const
    {
        DoQSort( &worker, lessAt, swapAt);
    }

    //---------------------------------------------------------------------------------------------
    // Search Algorithms

template < typename LessFn>
    TSzType LowerBound( LessFn&& lessFn) const
    {
        TSzType         l = _First;
        TSzType         h = static_cast< TSzType>( _First + Size());
        while ( l < h) {
            TSzType     mid = static_cast< TSzType>( l + ( h - l) / 2);
            if ( lessFn( mid))
                l = static_cast< TSzType>( mid + 1);
            else
                h = mid;
        }
        return l;
    }

template < typename LessFn>
    TSzType UpperBound( LessFn&& lessFn) const
    {
        TSzType         l = _First;
        TSzType         h = static_cast< TSzType>( _First + Size());
        while ( l < h) {
            TSzType     mid = static_cast< TSzType>( l + ( h - l) / 2);
            if ( lessFn( mid))
                h = mid;
            else
                l = static_cast< TSzType>( mid + 1);
        }
        return l;
    }

template < typename LessFn>
    Seg LocateBound( LessFn&& lessFn) const
    {
        TSzType         lo = LowerBound( lessFn);
        TSzType         hi = UpperBound( lessFn);
        return Seg( lo, static_cast< TSzType>( hi - lo));
    }

template < typename CmpFn>
    SearchResult BinarySearch( CmpFn&& cmpFn) const
    {
        TSzType         l = _First;
        TSzType         h = static_cast< TSzType>( _First + Size());
        while ( l < h) {
            TSzType     mid = static_cast< TSzType>( l + ( h - l) / 2);
            auto        cmp = cmpFn( mid);
            if ( cmp < 0)
                l = static_cast< TSzType>( mid + 1);
            else if ( cmp > 0)
                h = mid;
            else
                return { true, mid };
        }
        return { false, l };
    }

    //---------------------------------------------------------------------------------------------
    // Comparison & Range-based For Loop Support

    constexpr bool operator==( const Seg& o) const noexcept = default;

    struct Iterator
    {
        TSzType         _Current{0};

        constexpr TSzType operator*( void) const noexcept
        {
            return _Current;
        }

        constexpr Iterator& operator++( void) noexcept
        {
            ++_Current;
            return *this;
        }

        constexpr Iterator operator++( int) noexcept
        {
            Iterator    tmp = *this;
            ++_Current;
            return tmp;
        }

        constexpr bool operator==( const Iterator& o) const noexcept = default;
    };

    constexpr Iterator begin( void) const noexcept
    {
        return { _First };
    }

    constexpr Iterator end( void) const noexcept
    {
        return { End() };
    }

    constexpr Iterator cbegin( void) const noexcept
    {
        return begin();
    }

    constexpr Iterator cend( void) const noexcept
    {
        return end();
    }
};

//-------------------------------------------------------------------------------------------------

using USeg = Seg< uint32_t>;

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_SEG_H
