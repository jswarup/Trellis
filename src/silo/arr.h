// arr.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "silo/seg.h"
#include "silo/access.h"
#include "silo/traits.h"

#include <string_view>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

template < typename TElem>
class Arr;

//-------------------------------------------------------------------------------------------------
// Concept: CArrAccessible — validates that a container provides mutable indexed access, data pointer, and size.

template < typename T, typename TElem>
concept CArrAccessible = requires( T& c, uint32_t k) {
    { c.size() } -> std::convertible_to< size_t>;
    { c[k] }     -> std::convertible_to< TElem&>;
    { c.data() } -> std::convertible_to< TElem*>;
} || requires( T& c, uint32_t k) {
    { c.Size() } -> std::convertible_to< uint32_t>;
    { c[k] }     -> std::convertible_to< TElem&>;
    { c.Data() } -> std::convertible_to< TElem*>;
};

//-------------------------------------------------------------------------------------------------
// ArrTrait — zero-virtual trait definition for mutable indexed buffer access.

template < typename TElem>
struct ArrTrait
{
    struct VTable
    {
        uint32_t        (*Size)( const void* self);
        TElem*          (*Data)( void* self);
    };

template < typename TContainer>
        requires CArrAccessible< TContainer, TElem>
    static constexpr VTable Bind( void) noexcept
    {
        return {
            .Size = []( const void* s) -> uint32_t {
                const auto&     c = *static_cast< const TContainer*>( s);
                if constexpr ( requires { { c.Size() } -> std::convertible_to< uint32_t>; }) {
                    return static_cast< uint32_t>( c.Size());
                } else {
                    return static_cast< uint32_t>( c.size());
                }
            },
            .Data = []( void* s) -> TElem* {
                auto&           c = *static_cast< TContainer*>( s);
                if constexpr ( requires { { c.Data() } -> std::convertible_to< TElem*>; }) {
                    return c.Data();
                } else if constexpr ( requires { { c.data() } -> std::convertible_to< TElem*>; }) {
                    return c.data();
                } else {
                    return &c[0];
                }
            }
        };
    }
};

//-------------------------------------------------------------------------------------------------
// IArr — Trait declaring and implementing rich mutable array methods with zero data members.

template < typename TDerivedOrElem, typename TElem = void>
class IArr;

// Specialization 1: Trait Mixin for static polymorphism (inherits read-only IAccess trait)
template < typename TDerived, typename TElem>
class IArr : public IAccess< TDerived, TElem>
{
    constexpr const TDerived& Self( void) const noexcept
    {
        return static_cast< const TDerived&>( *this);
    }

    constexpr TDerived& Self( void) noexcept
    {
        return static_cast< TDerived&>( *this);
    }

public:
    using IAccess< TDerived, TElem>::operator[];
    using IAccess< TDerived, TElem>::First;
    using IAccess< TDerived, TElem>::Last;
    using IAccess< TDerived, TElem>::begin;
    using IAccess< TDerived, TElem>::end;

    constexpr TElem& operator[]( uint32_t k) const noexcept
    {
        return Self().Data()[k];
    }

    constexpr TElem& First( void) const noexcept
    {
        return Self().Data()[0];
    }

    constexpr TElem& Last( void) const noexcept
    {
        return Self().Data()[Self().Size() - 1];
    }

    constexpr const TElem& SetAt( uint32_t k, const TElem& val) const
    {
        Self().Data()[k] = val;
        return Self().Data()[k];
    }

    constexpr const TElem& SwapAt( uint32_t k, TElem& val) const
    {
        std::swap( Self().Data()[k], val);
        return Self().Data()[k];
    }

    constexpr void Swap( uint32_t i, uint32_t j) const noexcept
    {
        std::swap( Self().Data()[i], Self().Data()[j]);
    }

    constexpr void SwapFrom( uint32_t dstStart, const Arr< TElem>& src, uint32_t srcStart, uint32_t count) const noexcept
    {
        TElem*          d = Self().Data();
        for ( uint32_t i = 0; i < count; ++i)
            std::swap( d[dstStart + i], src[srcStart + i]);
    }

    constexpr Arr< TElem> LSnip( uint32_t count) const noexcept
    {
        const uint32_t  sz = Self().Size();
        const uint32_t  snip = ( count < sz) ? count : sz;
        return { Self().Data() + snip, sz - snip };
    }

    constexpr Arr< TElem> RSnip( uint32_t count) const noexcept
    {
        const uint32_t  sz = Self().Size();
        const uint32_t  newSize = ( sz > count) ? ( sz - count) : 0;
        return { Self().Data(), newSize };
    }

    constexpr Arr< TElem> Subset( uint32_t first, uint32_t sz) const noexcept
    {
        return { Self().Data() + first, sz };
    }

    constexpr void DoIndexSetup( void) const
        requires std::constructible_from< TElem, size_t>
    {
        const uint32_t  sz = Self().Size();
        for ( uint32_t i = 0; i < sz; ++i)
            SetAt( i, static_cast< TElem>( i));
    }

    std::string_view AsStringView( void) const noexcept
        requires ( std::same_as< std::remove_const_t< TElem>, char> || std::same_as< std::remove_const_t< TElem>, uint8_t>)
    {
        return std::string_view( reinterpret_cast< const char*>( Self().Data()), Self().Size());
    }

    constexpr TElem* begin( void) const noexcept
    {
        return Self().Data();
    }

    constexpr TElem* end( void) const noexcept
    {
        return Self().Data() + Self().Size();
    }
};

//-------------------------------------------------------------------------------------------------
// Arr — non-owning, contiguous, mutable slice fat array implementing IArr.
// Exactly 16 bytes, trivially copyable, zero-virtual.

template < typename TElem>
class Arr : public IArr< Arr< TElem>, TElem>
{
public:
    using value_type      = TElem;
    using size_type       = uint32_t;
    using difference_type = ptrdiff_t;
    using reference       = TElem&;
    using const_reference = const TElem&;
    using pointer         = TElem*;
    using const_pointer   = const TElem*;

protected:
    TElem*              _Ptr{nullptr};
    uint32_t            _Size{0};

public:
    constexpr Arr( void) noexcept = default;

    constexpr Arr( TElem* ptr, uint32_t size) noexcept
        : _Ptr( ptr),
          _Size( size)
    {
    }

template < typename TOther>
        requires ( std::is_const_v< TElem> && std::same_as< const TOther, TElem>)
    constexpr Arr( const Arr< TOther>& other) noexcept
        : _Ptr( other.Data()),
          _Size( other.Size())
    {
    }

template < typename TRange>
        requires ( !std::same_as< std::decay_t< TRange>, Arr> &&
                   requires( TRange&& r) {
                       { std::data( r) } -> std::convertible_to< const TElem*>;
                       { std::size( r) } -> std::convertible_to< size_t>;
                   })
    constexpr Arr( TRange&& r) noexcept
        : _Ptr( const_cast< TElem*>( std::data( r))),
          _Size( static_cast< uint32_t>( std::size( r)))
    {
    }

    constexpr TElem* Data( void) const noexcept
    {
        return _Ptr;
    }

    constexpr uint32_t Size( void) const noexcept
    {
        return _Size;
    }

    constexpr TElem* data( void) const noexcept
    {
        return _Ptr;
    }

    constexpr uint32_t size( void) const noexcept
    {
        return _Size;
    }

    constexpr bool operator==( const Arr& o) const noexcept
    {
        if ( _Size != o._Size)
            return false;

        for ( uint32_t i = 0; i < _Size; ++i) {
            if ( _Ptr[i] != o._Ptr[i])
                return false;
        }
        return true;
    }

    IAccess< TElem> AsAccess( void) const noexcept
    {
        return IAccess< TElem>( *this);
    }
};

//-------------------------------------------------------------------------------------------------
// Specialization 2: IArr<TElem> — Dynamic Trait Facade (fat pointer over MTRef<ArrTrait<TElem>>)

template < typename TElem>
class IArr< TElem, void> : public MTRef< ArrTrait< TElem>>, public IArr< IArr< TElem, void>, TElem>
{
public:
    using Base = MTRef< ArrTrait< TElem>>;
    using Base::Base;

    constexpr IArr( const Base& ref) noexcept
        : Base( ref)
    {
    }

    constexpr IArr( Arr< TElem>& arr) noexcept
        : Base( arr)
    {
    }

    constexpr uint32_t Size( void) const noexcept
    {
        return this->_Ops ? this->_Ops->Size( this->_Ptr) : 0;
    }

    constexpr TElem* Data( void) const noexcept
    {
        return this->_Ops ? this->_Ops->Data( this->_Ptr) : nullptr;
    }

    IAccess< TElem> AsAccess( void) const noexcept
    {
        return IAccess< TElem>( *this);
    }
};

//-------------------------------------------------------------------------------------------------

template < typename TElem>
using ArrRef = IArr< TElem>;

} // namespace trellis::silo
