#ifndef TRELLIS_SILO_STASH_H
#define TRELLIS_SILO_STASH_H

//-------------------------------------------------------------------------------------------------

#include "silo/access.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/seg.h"
#include "silo/stk.h"
#include "stalks/atm.h"

#include <initializer_list>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Stash — high-performance dynamic array container providing 32-bit indexing, zero-copy slicing,
// and lock-free atomic stack capabilities (Stk) backed by an owning Buff.
// Modeled directly from Kosh silo/stash.rs. Exactly 24 bytes, zero-virtual.

template < typename TElem>
class Stash
{
public:
    using value_type      = TElem;
    using size_type       = uint32_t;
    using difference_type = ptrdiff_t;
    using reference       = TElem&;
    using const_reference = const TElem&;
    using pointer         = TElem*;
    using const_pointer   = const TElem*;
    using iterator        = TElem*;
    using const_iterator  = const TElem*;

private:
    Buff< TElem>                    _Buff{};
    mutable stalks::Atm< uint32_t>  _Sz{0};

public:
    //---------------------------------------------------------------------------------------------
    // Constructors & Assignment

    constexpr Stash( void) noexcept = default;

    explicit Stash( uint32_t count, const TElem& val = TElem())
        requires std::is_copy_constructible_v< TElem>
        : _Buff( count, val),
          _Sz( count)
    {
    }

    Stash( uint32_t capacity, uint32_t initialSize, const TElem& def)
        requires std::is_copy_constructible_v< TElem>
        : _Buff( capacity, def),
          _Sz( initialSize)
    {
    }

template < typename Dispenser>
        requires std::is_invocable_r_v< TElem, Dispenser, uint32_t>
    Stash( uint32_t capacity, uint32_t initialSize, Dispenser&& dispenser)
        : _Buff( capacity, std::forward< Dispenser>( dispenser)),
          _Sz( initialSize)
    {
    }

    Stash( std::initializer_list< TElem> init)
        requires std::is_copy_constructible_v< TElem>
        : _Buff( init),
          _Sz( static_cast< uint32_t>( init.size()))
    {
    }

    Stash( Arr< const TElem> arr)
        requires std::is_copy_constructible_v< TElem>
        : _Buff( arr),
          _Sz( arr.Size())
    {
    }

    Stash( const Stash& other)
        requires std::is_copy_constructible_v< TElem>
        : _Buff( other.AsArr()),
          _Sz( other.Size())
    {
    }

    Stash& operator=( const Stash& other)
        requires std::is_copy_constructible_v< TElem>
    {
        if ( this != &other) {
            _Buff = Buff< TElem>( other.AsArr());
            _Sz.Store( other.Size(), std::memory_order_release);
        }
        return *this;
    }

    Stash( Stash&& other) noexcept
        : _Buff( std::move( other._Buff)),
          _Sz( other._Sz.Exchange( 0, std::memory_order_acq_rel))
    {
    }

    Stash& operator=( Stash&& other) noexcept
    {
        if ( this != &other) {
            _Buff = std::move( other._Buff);
            _Sz.Store( other._Sz.Exchange( 0, std::memory_order_acq_rel), std::memory_order_release);
        }
        return *this;
    }

    Stash& operator=( std::initializer_list< TElem> init)
        requires std::is_copy_constructible_v< TElem>
    {
        _Buff = Buff< TElem>( init);
        _Sz.Store( static_cast< uint32_t>( init.size()), std::memory_order_release);
        return *this;
    }

    Stash& operator=( Arr< const TElem> arr)
        requires std::is_copy_constructible_v< TElem>
    {
        _Buff = Buff< TElem>( arr);
        _Sz.Store( arr.Size(), std::memory_order_release);
        return *this;
    }

    //---------------------------------------------------------------------------------------------
    // Factory Methods

    static Stash New( uint32_t capacity, uint32_t initialSize, const TElem& def)
        requires std::is_copy_constructible_v< TElem>
    {
        return Stash( capacity, initialSize, def);
    }

    static constexpr Stash NewEmpty( void) noexcept
    {
        return {};
    }

template < typename Dispenser>
        requires std::is_invocable_r_v< TElem, Dispenser, uint32_t>
    static Stash Create( uint32_t capacity, uint32_t initialSize, Dispenser&& dispenser)
    {
        return Stash( capacity, initialSize, std::forward< Dispenser>( dispenser));
    }

    //---------------------------------------------------------------------------------------------
    // Size & Capacity Queries

    uint32_t Size( void) const noexcept
    {
        return _Sz.Load( std::memory_order_acquire);
    }

    uint32_t Capacity( void) const noexcept
    {
        return _Buff.Size();
    }

    bool IsEmpty( void) const noexcept
    {
        return Size() == 0;
    }

    //---------------------------------------------------------------------------------------------
    // Dynamic Growth & Capacity Operations

    void Reserve( uint32_t newCap)
    {
        if ( newCap > _Buff.Size()) {
            _Buff.Resize( newCap, []( uint32_t) {
                return TElem();
            });
        }
    }

    void Resize( uint32_t newSize, const TElem& def = TElem())
        requires std::is_copy_constructible_v< TElem>
    {
        if ( newSize > _Buff.Size())
            Reserve( newSize);

        const uint32_t  oldSize = Size();
        for ( uint32_t i = oldSize; i < newSize; ++i)
            _Buff[i] = def;

        _Sz.Store( newSize, std::memory_order_release);
    }

    void Clear( void) noexcept
    {
        _Sz.Store( 0, std::memory_order_release);
    }

    void TrimBuff( void)
    {
        const uint32_t  curSz = Size();
        if ( curSz < _Buff.Size())
            _Buff.ShrinkTo( curSz);
    }

    void ShrinkToFit( void)
    {
        TrimBuff();
    }

    Buff< TElem> ExtractBuff( bool shrinkToFit = true)
    {
        const uint32_t  curSz = Size();
        if ( shrinkToFit && curSz < _Buff.Size())
            _Buff.ShrinkTo( curSz);

        _Sz.Store( 0, std::memory_order_release);
        return std::move( _Buff);
    }

    //---------------------------------------------------------------------------------------------
    // Element Access

    TElem& operator[]( uint32_t index) noexcept
    {
        return _Buff[index];
    }

    const TElem& operator[]( uint32_t index) const noexcept
    {
        return _Buff[index];
    }

    TElem& At( uint32_t index)
    {
        return _Buff[index];
    }

    const TElem& At( uint32_t index) const
    {
        return _Buff[index];
    }

    TElem* Data( void) noexcept
    {
        return _Buff.Data();
    }

    const TElem* Data( void) const noexcept
    {
        return _Buff.Data();
    }

    TElem& Front( void) noexcept
    {
        return _Buff[0];
    }

    const TElem& Front( void) const noexcept
    {
        return _Buff[0];
    }

    TElem& Back( void) noexcept
    {
        return _Buff[Size() - 1];
    }

    const TElem& Back( void) const noexcept
    {
        return _Buff[Size() - 1];
    }

    //---------------------------------------------------------------------------------------------
    // Dynamic Element Insertion & Removal

    void PushBack( const TElem& val)
        requires std::is_copy_constructible_v< TElem>
    {
        const uint32_t  curSz = Size();
        if ( curSz >= _Buff.Size()) {
            const uint32_t  newCap = ( _Buff.Size() == 0) ? 4 : ( _Buff.Size() * 2);
            Reserve( newCap);
        }
        _Buff[curSz] = val;
        _Sz.Store( curSz + 1, std::memory_order_release);
    }

    void PushBack( TElem&& val)
    {
        const uint32_t  curSz = Size();
        if ( curSz >= _Buff.Size()) {
            const uint32_t  newCap = ( _Buff.Size() == 0) ? 4 : ( _Buff.Size() * 2);
            Reserve( newCap);
        }
        _Buff[curSz] = std::move( val);
        _Sz.Store( curSz + 1, std::memory_order_release);
    }

template < typename... TArgs>
    TElem& EmplaceBack( TArgs&&... args)
    {
        const uint32_t  curSz = Size();
        if ( curSz >= _Buff.Size()) {
            const uint32_t  newCap = ( _Buff.Size() == 0) ? 4 : ( _Buff.Size() * 2);
            Reserve( newCap);
        }
        _Buff[curSz] = TElem( std::forward< TArgs>( args)...);
        _Sz.Store( curSz + 1, std::memory_order_release);
        return _Buff[curSz];
    }

    bool PopBack( void) noexcept
    {
        const uint32_t  curSz = Size();
        if ( curSz > 0) {
            _Sz.Store( curSz - 1, std::memory_order_release);
            return true;
        }
        return false;
    }

    void Append( Arr< const TElem> slice)
        requires std::is_copy_constructible_v< TElem>
    {
        const uint32_t  addSz = slice.Size();
        if ( addSz == 0)
            return;

        const uint32_t  curSz = Size();
        const uint32_t  reqCap = curSz + addSz;
        if ( reqCap > _Buff.Size()) {
            uint32_t    newCap = ( _Buff.Size() == 0) ? 4 : _Buff.Size();
            while ( newCap < reqCap)
                newCap *= 2;
            Reserve( newCap);
        }
        for ( uint32_t i = 0; i < addSz; ++i)
            _Buff[curSz + i] = slice[i];

        _Sz.Store( reqCap, std::memory_order_release);
    }

    //---------------------------------------------------------------------------------------------
    // Iterators

    iterator begin( void) noexcept
    {
        return _Buff.Data();
    }

    iterator end( void) noexcept
    {
        return _Buff.Data() + Size();
    }

    const_iterator begin( void) const noexcept
    {
        return _Buff.Data();
    }

    const_iterator end( void) const noexcept
    {
        return _Buff.Data() + Size();
    }

    const_iterator cbegin( void) const noexcept
    {
        return _Buff.Data();
    }

    const_iterator cend( void) const noexcept
    {
        return _Buff.Data() + Size();
    }

    //---------------------------------------------------------------------------------------------
    // Slicing & Views

    operator Arr< TElem>( void) noexcept
    {
        return Arr< TElem>( Data(), Size());
    }

    operator Arr< const TElem>( void) const noexcept
    {
        return Arr< const TElem>( Data(), Size());
    }

    Arr< TElem> AsArr( void) noexcept
    {
        return Arr< TElem>( Data(), Size());
    }

    Arr< const TElem> AsArr( void) const noexcept
    {
        return Arr< const TElem>( Data(), Size());
    }

    USeg AsSeg( void) const noexcept
    {
        return silo::USeg( Size());
    }

    IArr< TElem> AsIArr( void) noexcept
    {
        return IArr< TElem>( AsArr());
    }

    IAccess< TElem> AsAccess( void) const noexcept
    {
        return IAccess< TElem>( AsArr());
    }

    //---------------------------------------------------------------------------------------------
    // Atomic Stack Compatibility (Stk)

    silo::Stk< TElem> StkView( void) const noexcept
    {
        return silo::Stk< TElem>( &_Sz, Arr< TElem>( const_cast< TElem*>( _Buff.Data()), _Buff.Size()));
    }

    silo::Stk< TElem> Stk( void) const noexcept
    {
        return StkView();
    }

    bool Pop( TElem& val) const
    {
        return StkView().Pop( val);
    }

    void Push( TElem val)
    {
        while ( !StkView().Push( val)) {
            if ( Size() == _Buff.Size()) {
                const uint32_t  newCap = ( _Buff.Size() == 0) ? 4 : ( _Buff.Size() * 2);
                _Buff.Resize( newCap, [&]( uint32_t) {
                    return val;
                });
            }
        }
    }

    void DoIndexSetup( void) const
        requires std::constructible_from< TElem, size_t>
    {
        _Buff.DoIndexSetup();
        _Sz.Store( _Buff.Size(), std::memory_order_release);
    }
};

} // namespace trellis::silo

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_SILO_STASH_H
