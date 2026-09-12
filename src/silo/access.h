// access.h -------------------------------------------------------------------------------------------------------
#pragma once

#include "silo/seg.h"
#include "silo/traits.h"

#include <string>
#include <iterator>
#include <format>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Concept: CAccessible — validates that a container/slice provides indexed access and length for TElem.

template < typename T, typename TElem>
concept CAccessible = requires( const T& c, uint32_t k) {
    { c.size() } -> std::convertible_to< size_t>;
    { c[k] }     -> std::convertible_to< const TElem&>;
} || requires( const T& c, uint32_t k) {
    { c.Size() } -> std::convertible_to< uint32_t>;
    { c.At( k) } -> std::convertible_to< const TElem&>;
} || requires( const T& c, uint32_t k) {
    { c.Size() } -> std::convertible_to< uint32_t>;
    { c[k] }     -> std::convertible_to< const TElem&>;
};

//-------------------------------------------------------------------------------------------------
// AccessTrait — zero-virtual trait definition for indexed read-only element access.

template < typename TElem>
struct AccessTrait
{
    struct VTable
    {
        uint32_t        (*Size)( const void* self);
        const TElem*    (*At)( const void* self, uint32_t k);
    };

template < typename TContainer>
        requires CAccessible< TContainer, TElem>
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
            .At = []( const void* s, uint32_t k) -> const TElem* {
                const auto&     c = *static_cast< const TContainer*>( s);
                if constexpr ( requires { { c.At( k) } -> std::convertible_to< const TElem&>; }) {
                    return &c.At( k);
                } else {
                    return &c[k];
                }
            }
        };
    }
};

//-------------------------------------------------------------------------------------------------
// IAccess — Trait declaring and implementing read-only indexed buffer operations with zero data members.

template < typename TDerivedOrElem, typename TElem = void>
class IAccess;

// Specialization 1: Trait Mixin / Interface for static polymorphism (zero data members)
template < typename TDerived, typename TElem>
class IAccess
{
    constexpr const TDerived& Self( void) const noexcept
    {
        return static_cast< const TDerived&>( *this);
    }

public:
    constexpr uint32_t Size( void) const noexcept
    {
        return Self().Size();
    }

    constexpr bool IsEmpty( void) const noexcept
    {
        return Self().Size() == 0;
    }

    constexpr const TElem& operator[]( uint32_t k) const noexcept
    {
        return Self().Data()[k];
    }

    constexpr const TElem& At( uint32_t k) const noexcept
    {
        return Self().Data()[k];
    }

    constexpr const TElem& First( void) const noexcept
    {
        return Self().Data()[0];
    }

    constexpr const TElem& Last( void) const noexcept
    {
        return Self().Data()[Self().Size() - 1];
    }

    constexpr silo::USeg USeg( void) const noexcept
    {
        return silo::USeg( Self().Size());
    }

template < typename F>
    constexpr bool Span( F&& f) const
    {
        const uint32_t  sz = Self().Size();
        for ( uint32_t k = 0; k < sz; ++k) {
            if ( !f( Self().Data()[k]))
                return false;
        }
        return true;
    }

template < typename F>
    constexpr void Traverse( F&& f) const
    {
        const uint32_t  sz = Self().Size();
        for ( uint32_t k = 0; k < sz; ++k)
            f( Self().Data()[k]);
    }

template < typename Less>
    constexpr bool SortSanity( Less&& less) const
    {
        const uint32_t  sz = Self().Size();
        if ( sz <= 1)
            return true;

        for ( uint32_t k = 0; k < sz - 1; ++k) {
            if ( less( Self().Data()[k + 1], Self().Data()[k]))
                return false;
        }
        return true;
    }

    std::string Format( void) const
    {
        std::string     s = "[";
        const uint32_t  sz = Self().Size();
        for ( uint32_t i = 0; i < sz; ++i) {
            if ( i > 0)
                s += ", ";

            if constexpr ( requires { std::to_string( Self().Data()[i]); }) {
                s += std::to_string( Self().Data()[i]);
            } else if constexpr ( requires { std::format( "{}", Self().Data()[i]); }) {
                s += std::format( "{}", Self().Data()[i]);
            }
        }
        s += "]";
        return s;
    }

    constexpr const TElem* begin( void) const noexcept
    {
        return Self().Data();
    }

    constexpr const TElem* end( void) const noexcept
    {
        return Self().Data() + Self().Size();
    }

    constexpr const TElem* cbegin( void) const noexcept
    {
        return begin();
    }

    constexpr const TElem* cend( void) const noexcept
    {
        return end();
    }
};

// Specialization 2: Dynamic Trait Facade (fat pointer over TRef<AccessTrait<TElem>>)
template < typename TElem>
class IAccess< TElem, void> : public TRef< AccessTrait< TElem>>
{
public:
    using Base = TRef< AccessTrait< TElem>>;
    using Base::Base;

    constexpr IAccess( const Base& ref) noexcept
        : Base( ref)
    {
    }

    constexpr uint32_t Size( void) const noexcept
    {
        return this->_Ops ? this->_Ops->Size( this->_Ptr) : 0;
    }

    constexpr const TElem& operator[]( uint32_t k) const noexcept
    {
        return *( this->_Ops->At( this->_Ptr, k));
    }

    constexpr const TElem& At( uint32_t k) const noexcept
    {
        return ( *this)[k];
    }

    constexpr bool IsEmpty( void) const noexcept
    {
        return Size() == 0;
    }

    constexpr const TElem& First( void) const noexcept
    {
        return ( *this)[0];
    }

    constexpr const TElem& Last( void) const noexcept
    {
        return ( *this)[Size() - 1];
    }

    constexpr silo::USeg USeg( void) const noexcept
    {
        return silo::USeg( Size());
    }

template < typename F>
    constexpr bool Span( F&& f) const
    {
        return USeg().Span( [&]( uint32_t k) {
            return f( ( *this)[k]);
        });
    }

template < typename F>
    constexpr void Traverse( F&& f) const
    {
        USeg().Traverse( [&]( uint32_t k) {
            f( ( *this)[k]);
        });
    }

template < typename Less>
    constexpr bool SortSanity( Less&& less) const
    {
        return USeg().RSnip( 1).Span( [&]( uint32_t k) {
            return !less( ( *this)[k + 1], ( *this)[k]);
        });
    }

    std::string Format( void) const
    {
        std::string     s = "[";
        const uint32_t  sz = Size();
        for ( uint32_t i = 0; i < sz; ++i) {
            if ( i > 0)
                s += ", ";

            if constexpr ( requires { std::to_string( ( *this)[i]); }) {
                s += std::to_string( ( *this)[i]);
            } else if constexpr ( requires { std::format( "{}", ( *this)[i]); }) {
                s += std::format( "{}", ( *this)[i]);
            }
        }
        s += "]";
        return s;
    }

    //---------------------------------------------------------------------------------------------
    // Random Access Iterator supporting range-based for loops and standard algorithms

    class Iterator
    {
    private:
        const IAccess*  _Access{nullptr};
        uint32_t        _Index{0};

    public:
        using iterator_category = std::random_access_iterator_tag;
        using value_type        = TElem;
        using difference_type   = ptrdiff_t;
        using pointer           = const TElem*;
        using reference         = const TElem&;

        constexpr Iterator( void) noexcept = default;

        constexpr Iterator( const IAccess* access, uint32_t index) noexcept
            : _Access( access),
              _Index( index)
        {
        }

        constexpr reference operator*( void) const noexcept
        {
            return ( *_Access)[_Index];
        }

        constexpr pointer operator->( void) const noexcept
        {
            return &( *_Access)[_Index];
        }

        constexpr Iterator& operator++( void) noexcept
        {
            ++_Index;
            return *this;
        }

        constexpr Iterator operator++( int) noexcept
        {
            Iterator tmp = *this;
            ++_Index;
            return tmp;
        }

        constexpr Iterator& operator--( void) noexcept
        {
            --_Index;
            return *this;
        }

        constexpr Iterator operator--( int) noexcept
        {
            Iterator tmp = *this;
            --_Index;
            return tmp;
        }

        constexpr Iterator& operator+=( difference_type n) noexcept
        {
            _Index += static_cast< uint32_t>( n);
            return *this;
        }

        constexpr Iterator& operator-=( difference_type n) noexcept
        {
            _Index -= static_cast< uint32_t>( n);
            return *this;
        }

        constexpr Iterator operator+( difference_type n) const noexcept
        {
            return { _Access, _Index + static_cast< uint32_t>( n) };
        }

        constexpr Iterator operator-( difference_type n) const noexcept
        {
            return { _Access, _Index - static_cast< uint32_t>( n) };
        }

        constexpr difference_type operator-( const Iterator& o) const noexcept
        {
            return static_cast< difference_type>( _Index) - static_cast< difference_type>( o._Index);
        }

        constexpr reference operator[]( difference_type n) const noexcept
        {
            return ( *_Access)[_Index + static_cast< uint32_t>( n)];
        }

        constexpr bool operator==( const Iterator& o) const noexcept
        {
            return _Access == o._Access && _Index == o._Index;
        }

        constexpr bool operator!=( const Iterator& o) const noexcept
        {
            return !( *this == o);
        }

        constexpr auto operator<=>( const Iterator& o) const noexcept
        {
            return _Index <=> o._Index;
        }
    };

    constexpr Iterator begin( void) const noexcept
    {
        return { this, 0 };
    }

    constexpr Iterator end( void) const noexcept
    {
        return { this, Size() };
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

} // namespace trellis::silo
