// fifo.h ---------------------------------------------------------------------------------------------------------
#pragma once

#include <cstdint>
#include <cstddef>
#include <type_traits>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Fifo — static, fixed-size circular buffer queue.
// Designed for cycle-accurate hardware simulation FIFOs.
// Zero dynamic allocations, trivially copyable if T is trivial.

template < typename TElem, uint32_t TCap>
class Fifo
{
    static_assert( TCap > 0, "silo::Fifo capacity must be > 0");

private:
    TElem       _Data[TCap]{};
    uint32_t    _Head{0};
    uint32_t    _Tail{0};
    uint32_t    _Size{0};

public:
    constexpr Fifo( void) noexcept = default;

    constexpr bool IsEmpty( void) const noexcept { return _Size == 0; }
    constexpr bool IsFull( void) const noexcept  { return _Size == TCap; }
    constexpr uint32_t Size( void) const noexcept { return _Size; }
    constexpr uint32_t Capacity( void) const noexcept { return TCap; }

    void PushBack( const TElem& val) noexcept
    {
        if ( _Size < TCap) {
            _Data[_Tail] = val;
            _Tail = ( _Tail + 1) % TCap;
            _Size++;
        }
    }

    void PopFront( void) noexcept
    {
        if ( _Size > 0) {
            _Head = ( _Head + 1) % TCap;
            _Size--;
        }
    }

    TElem& Front( void) noexcept
    {
        return _Data[_Head];
    }

    const TElem& Front( void) const noexcept
    {
        return _Data[_Head];
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::silo

