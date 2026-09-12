#ifndef TRELLIS_RUBE_REG_H
#define TRELLIS_RUBE_REG_H

//-------------------------------------------------------------------------------------------------

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------
// Unified bit-packed register value with 4-state logic (0, 1, X, Z/I).
// Modeled directly from Kosh rube/reg.rs.

struct Reg
{
    uint64_t    _Val{0};
    bool        _X{true};
    bool        _I{false};

    constexpr Reg( void) noexcept = default;

    constexpr Reg( uint64_t val, bool x, bool i) noexcept
        : _Val( val), _X( x), _I( i)
    {
    }

    static const Reg TRUE;
    static const Reg FALSE;
    static const Reg X;
    static const Reg Z;
    static const Reg I;

    static constexpr Reg Known( uint64_t val) noexcept
    {
        return Reg{val, false, false};
    }

    static constexpr Reg Unknown( uint64_t /*dummy*/ = 0) noexcept
    {
        return Reg{0, true, false};
    }

    static constexpr Reg HighZ( uint64_t /*dummy*/ = 0) noexcept
    {
        return Reg{0, false, true};
    }

    constexpr uint64_t Val( void) const noexcept
    {
        return _Val;
    }

    constexpr bool IsX( void) const noexcept
    {
        return _X;
    }

    constexpr bool IsZ( void) const noexcept
    {
        return _I;
    }

    constexpr bool IsI( void) const noexcept
    {
        return _I;
    }

    constexpr bool IsValid( void) const noexcept
    {
        return !_X && !_I;
    }

    constexpr bool IsTrue( void) const noexcept
    {
        return !_X && !_I && ( ( _Val & 1) != 0);
    }

    constexpr bool IsFalse( void) const noexcept
    {
        return !_X && !_I && ( ( _Val & 1) == 0);
    }

    constexpr Reg AsBool( void) const noexcept
    {
        if ( _I) return HighZ();
        if ( _X) return Unknown();
        return ( ( _Val & 1) != 0) ? Known( 1) : Known( 0);
    }

    constexpr Reg Masked( uint64_t mask) const noexcept
    {
        return Reg{_Val & mask, _X, _I};
    }

    static constexpr Reg FromBool( bool val) noexcept
    {
        return val ? Known( 1) : Known( 0);
    }

    static constexpr Reg FromU32( uint32_t val) noexcept
    {
        return Known( static_cast< uint64_t>( val));
    }

    static constexpr Reg FromU64( uint64_t val) noexcept
    {
        return Known( val);
    }

    constexpr bool operator==( const Reg& other) const noexcept
    {
        if ( _X || other._X) return _X && other._X;
        if ( _I || other._I) return _I && other._I;
        return _Val == other._Val;
    }

    constexpr bool operator!=( const Reg& other) const noexcept
    {
        return !( *this == other);
    }

    constexpr Reg operator~( void) const noexcept
    {
        if ( _X || _I) return Unknown();
        if ( _Val == 1) return Known( 0);
        if ( _Val == 0) return Known( 1);
        return Reg{~_Val, false, false};
    }

    constexpr Reg operator&( const Reg& rhs) const noexcept
    {
        if ( IsFalse() || rhs.IsFalse()) return Known( 0);
        if ( IsX() || IsI() || rhs.IsX() || rhs.IsI()) return Unknown();
        return Reg{_Val & rhs._Val, false, false};
    }

    constexpr Reg operator|( const Reg& rhs) const noexcept
    {
        if ( IsTrue() || rhs.IsTrue()) return Known( 1);
        if ( IsX() || IsI() || rhs.IsX() || rhs.IsI()) return Unknown();
        return Reg{_Val | rhs._Val, false, false};
    }

    constexpr Reg operator^( const Reg& rhs) const noexcept
    {
        if ( IsX() || IsI() || rhs.IsX() || rhs.IsI()) return Unknown();
        return Reg{_Val ^ rhs._Val, false, false};
    }
};

inline constexpr Reg Reg::TRUE  = Reg{1, false, false};
inline constexpr Reg Reg::FALSE = Reg{0, false, false};
inline constexpr Reg Reg::X     = Reg{0, true,  false};
inline constexpr Reg Reg::Z     = Reg{0, false, true};
inline constexpr Reg Reg::I     = Reg::Z;

} // namespace trellis::rube

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_RUBE_REG_H

