#ifndef TRELLIS_RUBE_TRIGGER_H
#define TRELLIS_RUBE_TRIGGER_H

//-------------------------------------------------------------------------------------------------

#include "rube/reg.h"
#include "silo/buff.h"
#include "silo/seg.h"

#include <cstdint>
#include <cstring>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------

using TriggerId = uint32_t;

inline constexpr uint8_t PAST_X    = 1u << 0;
inline constexpr uint8_t PAST_I    = 1u << 1;
inline constexpr uint8_t PAST_MASK = 0b0000'0011;

inline constexpr uint8_t CURR_X    = 1u << 2;
inline constexpr uint8_t CURR_I    = 1u << 3;
inline constexpr uint8_t CURR_MASK = 0b0000'1100;

inline constexpr uint8_t FUTR_X    = 1u << 4;
inline constexpr uint8_t FUTR_I    = 1u << 5;
inline constexpr uint8_t FUTR_MASK = 0b0011'0000;

//-------------------------------------------------------------------------------------------------
// Hot temporal state cell for triggers in 4-Buff SoA layout.
// Modeled directly from Kosh rube/trigger.rs.

template < typename T = uint64_t>
class TriggerWad
{
public:
    silo::Buff< T>          _PastVals{};
    silo::Buff< T>          _CurrentVals{};
    silo::Buff< T>          _FutureVals{};
    silo::Buff< uint8_t>    _Flags{};
    silo::Buff< silo::USeg> _SubscriberSpans{};
    silo::Buff< uint32_t>   _Subscribers{};

public:
    constexpr TriggerWad( void) noexcept = default;

    TriggerWad(
        silo::Buff< T> pastVals,
        silo::Buff< T> currentVals,
        silo::Buff< T> futureVals,
        silo::Buff< uint8_t> flags,
        silo::Buff< silo::USeg> subscriberSpans,
        silo::Buff< uint32_t> subscribers)
        : _PastVals( std::move( pastVals)),
          _CurrentVals( std::move( currentVals)),
          _FutureVals( std::move( futureVals)),
          _Flags( std::move( flags)),
          _SubscriberSpans( std::move( subscriberSpans)),
          _Subscribers( std::move( subscribers))
    {
    }

    uint32_t Size( void) const noexcept
    {
        return _PastVals.Size();
    }

    T PastVal( TriggerId idx) const noexcept
    {
        return _PastVals[idx];
    }

    T CurrentVal( TriggerId idx) const noexcept
    {
        return _CurrentVals[idx];
    }

    T FutureVal( TriggerId idx) const noexcept
    {
        return _FutureVals[idx];
    }

    void SetFutureVal( TriggerId idx, T val) noexcept
    {
        _FutureVals[idx] = val;
        _Flags[idx] = static_cast< uint8_t>( _Flags[idx] & ~FUTR_MASK);
    }

    void SetImmediateVal( TriggerId idx, T val) noexcept
    {
        _CurrentVals[idx] = val;
        _FutureVals[idx] = val;
        _Flags[idx] = static_cast< uint8_t>( _Flags[idx] & ~( CURR_MASK | FUTR_MASK));
    }

    uint8_t Flags( TriggerId idx) const noexcept
    {
        return _Flags[idx];
    }

    std::pair< Reg, Reg> Advance( TriggerId idx) noexcept
    {
        const Reg past = Past( idx);
        const Reg current = Current( idx);
        _PastVals[idx] = _CurrentVals[idx];
        _CurrentVals[idx] = _FutureVals[idx];
        const uint8_t f = _Flags[idx];
        _Flags[idx] = static_cast< uint8_t>( ( ( f >> 2) & 0b0000'1111) | ( f & 0b0011'0000));
        return {past, current};
    }

    void AdvanceAll( void) noexcept
    {
        const uint32_t sz = Size();
        if ( sz > 0) {
            std::memcpy( _PastVals.Data(), _CurrentVals.Data(), sz * sizeof( T));
            std::memcpy( _CurrentVals.Data(), _FutureVals.Data(), sz * sizeof( T));
            for ( uint32_t i = 0; i < sz; ++i) {
                const uint8_t f = _Flags[i];
                _Flags[i] = static_cast< uint8_t>( ( ( f >> 2) & 0b0000'1111) | ( f & 0b0011'0000));
            }
        }
    }

    void Init( TriggerId idx, Reg val) noexcept
    {
        const T v = static_cast< T>( val._Val);
        _PastVals[idx] = v;
        _CurrentVals[idx] = v;
        _FutureVals[idx] = v;
        uint8_t f = 0;
        if ( val.IsX()) f |= ( PAST_X | CURR_X | FUTR_X);
        if ( val.IsI()) f |= ( PAST_I | CURR_I | FUTR_I);
        _Flags[idx] = f;
    }

    bool IsEdge( TriggerId idx) const noexcept
    {
        const T pastVal = _PastVals[idx];
        const T currVal = _CurrentVals[idx];
        const uint8_t f = _Flags[idx];
        const uint8_t pastFlags = f & PAST_MASK;
        const uint8_t currFlags = ( f >> 2) & PAST_MASK;
        return ( pastVal != currVal) || ( pastFlags != currFlags);
    }

    bool IsPosedge( TriggerId idx) const noexcept
    {
        const uint8_t f = _Flags[idx];
        if ( ( f & ( PAST_MASK | CURR_MASK)) != 0) {
            return false;
        }
        return ( ( static_cast< uint64_t>( _PastVals[idx]) & 1) == 0) &&
               ( ( static_cast< uint64_t>( _CurrentVals[idx]) & 1) != 0);
    }

    bool IsNegedge( TriggerId idx) const noexcept
    {
        const uint8_t f = _Flags[idx];
        if ( ( f & ( PAST_MASK | CURR_MASK)) != 0) {
            return false;
        }
        return ( ( static_cast< uint64_t>( _PastVals[idx]) & 1) != 0) &&
               ( ( static_cast< uint64_t>( _CurrentVals[idx]) & 1) == 0);
    }

    Reg Past( TriggerId idx) const noexcept
    {
        const uint64_t val = static_cast< uint64_t>( _PastVals[idx]);
        const uint8_t f = _Flags[idx];
        return Reg{val, ( f & PAST_X) != 0, ( f & PAST_I) != 0};
    }

    Reg Current( TriggerId idx) const noexcept
    {
        const uint64_t val = static_cast< uint64_t>( _CurrentVals[idx]);
        const uint8_t f = _Flags[idx];
        return Reg{val, ( f & CURR_X) != 0, ( f & CURR_I) != 0};
    }

    Reg Future( TriggerId idx) const noexcept
    {
        const uint64_t val = static_cast< uint64_t>( _FutureVals[idx]);
        const uint8_t f = _Flags[idx];
        return Reg{val, ( f & FUTR_X) != 0, ( f & FUTR_I) != 0};
    }

    void SetFuture( TriggerId idx, Reg val) noexcept
    {
        _FutureVals[idx] = static_cast< T>( val._Val);
        uint8_t f = static_cast< uint8_t>( _Flags[idx] & ~FUTR_MASK);
        if ( val.IsX()) f |= FUTR_X;
        if ( val.IsI()) f |= FUTR_I;
        _Flags[idx] = f;
    }

    void SetImmediate( TriggerId idx, Reg val) noexcept
    {
        const T v = static_cast< T>( val._Val);
        _CurrentVals[idx] = v;
        _FutureVals[idx] = v;
        uint8_t f = static_cast< uint8_t>( _Flags[idx] & ~( CURR_MASK | FUTR_MASK));
        if ( val.IsX()) f |= ( CURR_X | FUTR_X);
        if ( val.IsI()) f |= ( CURR_I | FUTR_I);
        _Flags[idx] = f;
    }
};

} // namespace trellis::rube

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_RUBE_TRIGGER_H

