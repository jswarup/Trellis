#ifndef TRELLIS_SILO_SEG_H
#define TRELLIS_SILO_SEG_H

//-------------------------------------------------------------------------------------------------

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Seg — integer segment range representation.

template < typename TSzType = uint32_t>
struct Seg
{
    TSzType             _Begin{0};
    TSzType             _Size{0};

    static constexpr Seg New( TSzType begin, TSzType size) noexcept
    {
        return { begin, size };
    }

    constexpr TSzType Begin( void) const noexcept
    {
        return _Begin;
    }

    constexpr TSzType End( void) const noexcept
    {
        return _Begin + _Size;
    }

    constexpr TSzType Size( void) const noexcept
    {
        return _Size;
    }

    constexpr bool IsEmpty( void) const noexcept
    {
        return _Size == 0;
    }

    constexpr Seg RSnip( TSzType count) const noexcept
    {
        return { _Begin, ( _Size > count) ? ( _Size - count) : 0 };
    }

    constexpr Seg LSnip( TSzType count) const noexcept
    {
        TSzType         snip = ( _Size > count) ? count : _Size;
        return { _Begin + snip, _Size - snip };
    }

    template < typename F>
    constexpr bool Span( F &&f) const
    {
        const TSzType   end = End();
        for ( TSzType i = _Begin; i < end; ++i) {
            if ( !f( i))
                return false;
        }
        return true;
    }

    template < typename F>
    constexpr void Traverse( F &&f) const
    {
        const TSzType   end = End();
        for ( TSzType i = _Begin; i < end; ++i)
            f( i);
    }
};

//-------------------------------------------------------------------------------------------------

using USeg = Seg< uint32_t>;

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_SEG_H

