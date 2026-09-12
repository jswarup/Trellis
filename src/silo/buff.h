#ifndef TRELLIS_SILO_BUFF_H
#define TRELLIS_SILO_BUFF_H

//-------------------------------------------------------------------------------------------------

#include "silo/arr.h"
#include "silo/access.h"
#include "silo/seg.h"
#include "silo/traits.h"

#include <cstdint>
#include <cstring>
#include <initializer_list>
#include <new>
#include <type_traits>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Buff — owning, heap-allocated, contiguous buffer derived directly from Arr.
// Exactly 16 bytes (inherits _Ptr and _Size), trivially moveable, zero-virtual.
// Modeled directly from Kosh silo/buff.rs.

template < typename TElem>
class Buff : public Arr< TElem>
{
public:
    using Base = Arr< TElem>;
    using typename Base::value_type;
    using typename Base::size_type;
    using typename Base::difference_type;
    using typename Base::reference;
    using typename Base::const_reference;
    using typename Base::pointer;
    using typename Base::const_pointer;

    //---------------------------------------------------------------------------------------------
    // Constructors & Destructor

    constexpr Buff( void) noexcept = default;

    Buff( uint32_t size, const TElem& initialValue)
        requires std::is_copy_constructible_v< TElem>
    {
        InitWith( size, [&]( TElem* p) {
            for ( uint32_t i = 0; i < size; ++i)
                ::new ( static_cast< void*>( p + i)) TElem( initialValue);
        });
    }

template < typename Dispenser>
        requires std::is_invocable_r_v< TElem, Dispenser, uint32_t>
    Buff( uint32_t size, Dispenser&& dispenser)
    {
        InitWith( size, [&]( TElem* p) {
            for ( uint32_t i = 0; i < size; ++i)
                ::new ( static_cast< void*>( p + i)) TElem( dispenser( i));
        });
    }

    Buff( std::initializer_list< TElem> init)
        requires std::is_copy_constructible_v< TElem>
        : Buff( Arr< const TElem>( init.begin(), static_cast< uint32_t>( init.size())))
    {
    }

    Buff( Arr< const TElem> arr)
        requires std::is_copy_constructible_v< TElem>
    {
        InitWith( arr.Size(), [&]( TElem* p) {
            for ( uint32_t i = 0; i < arr.Size(); ++i)
                ::new ( static_cast< void*>( p + i)) TElem( arr[i]);
        });
    }

    ~Buff( void) noexcept
    {
        Destroy();
    }

    Buff( const Buff& o)
        requires std::is_copy_constructible_v< TElem>
        : Buff( Arr< const TElem>( o._Ptr, o._Size))
    {
    }

    Buff& operator=( const Buff& o)
        requires std::is_copy_constructible_v< TElem>
    {
        if ( this != &o) {
            Buff        tmp( o);
            SwapBuff( tmp);
        }
        return *this;
    }

    Buff( Buff&& o) noexcept
        : Base( std::exchange( o._Ptr, nullptr), std::exchange( o._Size, 0))
    {
    }

    Buff& operator=( Buff&& o) noexcept
    {
        if ( this != &o) {
            Destroy();
            this->_Ptr  = std::exchange( o._Ptr, nullptr);
            this->_Size = std::exchange( o._Size, 0);
        }
        return *this;
    }

    //---------------------------------------------------------------------------------------------
    // Factory Methods

template < typename Dispenser>
        requires std::is_invocable_r_v< TElem, Dispenser, uint32_t>
    static Buff Create( uint32_t size, Dispenser&& dispenser)
    {
        return Buff( size, std::forward< Dispenser>( dispenser));
    }

    static Buff Concat( Arr< const TElem> a, Arr< const TElem> b)
        requires std::is_copy_constructible_v< TElem>
    {
        Buff            result;
        const uint32_t  aSz = a.Size();
        const uint32_t  bSz = b.Size();
        result.InitWith( aSz + bSz, [&]( TElem* p) {
            for ( uint32_t i = 0; i < aSz; ++i)
                ::new ( static_cast< void*>( p + i)) TElem( a[i]);
            for ( uint32_t i = 0; i < bSz; ++i)
                ::new ( static_cast< void*>( p + aSz + i)) TElem( b[i]);
        });
        return result;
    }

    //---------------------------------------------------------------------------------------------
    // Dynamic Operations

template < typename Dispenser>
        requires std::is_invocable_r_v< TElem, Dispenser, uint32_t>
    void Resize( uint32_t newSize, Dispenser&& dispenser)
    {
        if ( newSize <= this->_Size)
            return;

        const uint32_t  oldSize = this->_Size;
        GrowAndInit( newSize - oldSize, [&]( TElem* dst) {
            for ( uint32_t i = oldSize; i < newSize; ++i)
                ::new ( static_cast< void*>( dst + ( i - oldSize))) TElem( dispenser( i));
        });
    }

    void ExtendFromSlice( Arr< const TElem> slice)
        requires std::is_copy_constructible_v< TElem>
    {
        const uint32_t  addSize = slice.Size();
        if ( addSize == 0)
            return;

        GrowAndInit( addSize, [&]( TElem* dst) {
            if constexpr ( std::is_trivially_copyable_v< TElem>) {
                std::memcpy( dst, slice.Data(), addSize * sizeof( TElem));
            } else {
                for ( uint32_t i = 0; i < addSize; ++i)
                    ::new ( static_cast< void*>( dst + i)) TElem( slice[i]);
            }
        });
    }

    void SwapBuff( Buff& other) noexcept
    {
        std::swap( this->_Ptr, other._Ptr);
        std::swap( this->_Size, other._Size);
    }

    void ShrinkTo( uint32_t newSize)
    {
        if ( newSize >= this->_Size)
            return;

        if ( newSize == 0) {
            Destroy();
            return;
        }

        TElem*          newPtr = Allocate( newSize);
        if constexpr ( std::is_trivially_copyable_v< TElem>) {
            std::memcpy( newPtr, this->_Ptr, newSize * sizeof( TElem));
        } else {
            for ( uint32_t i = 0; i < newSize; ++i) {
                ::new ( static_cast< void*>( newPtr + i)) TElem( std::move( this->_Ptr[i]));
                this->_Ptr[i].~TElem();
            }
            for ( uint32_t i = newSize; i < this->_Size; ++i)
                this->_Ptr[i].~TElem();
        }

        if ( this->_Ptr)
            Deallocate( this->_Ptr);

        this->_Ptr  = newPtr;
        this->_Size = newSize;
    }

    void Trim( uint32_t newSize)
    {
        ShrinkTo( newSize);
    }

    //---------------------------------------------------------------------------------------------
    // Trait Facades & Views

    Arr< TElem> AsArr( void) noexcept
    {
        return Arr< TElem>( this->_Ptr, this->_Size);
    }

    Arr< const TElem> AsArr( void) const noexcept
    {
        return Arr< const TElem>( this->_Ptr, this->_Size);
    }

    IAccess< TElem> AsAccess( void) const noexcept
    {
        return IAccess< TElem>( *this);
    }

    IArr< TElem> AsIArr( void) noexcept
    {
        return IArr< TElem>( *this);
    }

private:
    static TElem* Allocate( uint32_t count)
    {
        return static_cast< TElem*>( ::operator new( count * sizeof( TElem), std::align_val_t{ alignof( TElem) }));
    }

    static void Deallocate( TElem* ptr) noexcept
    {
        ::operator delete( static_cast< void*>( ptr), std::align_val_t{ alignof( TElem) });
    }

template < typename FInit>
    void InitWith( uint32_t sz, FInit&& finit)
    {
        if ( sz > 0) {
            this->_Ptr  = Allocate( sz);
            this->_Size = sz;
            finit( this->_Ptr);
        }
    }

template < typename FInit>
    void GrowAndInit( uint32_t addCount, FInit&& finit)
    {
        const uint32_t  oldSize = this->_Size;
        const uint32_t  newSize = oldSize + addCount;
        TElem*          newPtr = Allocate( newSize);

        if constexpr ( std::is_trivially_copyable_v< TElem>) {
            if ( oldSize > 0 && this->_Ptr)
                std::memcpy( newPtr, this->_Ptr, oldSize * sizeof( TElem));
        } else {
            for ( uint32_t i = 0; i < oldSize; ++i) {
                ::new ( static_cast< void*>( newPtr + i)) TElem( std::move( this->_Ptr[i]));
                this->_Ptr[i].~TElem();
            }
        }
        finit( newPtr + oldSize);

        if ( this->_Ptr)
            Deallocate( this->_Ptr);

        this->_Ptr  = newPtr;
        this->_Size = newSize;
    }

    void Destroy( void) noexcept
    {
        if ( this->_Ptr) {
            if constexpr ( !std::is_trivially_destructible_v< TElem>) {
                for ( uint32_t i = 0; i < this->_Size; ++i)
                    this->_Ptr[i].~TElem();
            }
            Deallocate( this->_Ptr);
            this->_Ptr  = nullptr;
            this->_Size = 0;
        }
    }
};

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_BUFF_H

