// stk.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "silo/arr.h"
#include "silo/seg.h"
#include "stalks/atm.h"

#include <cstdint>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::silo {

//-------------------------------------------------------------------------------------------------
// Stk — non-owning atomic stack view over an Arr buffer and a shared atomic size counter.
// Exactly 24 bytes, zero-virtual, cache-friendly.
// Modeled directly from Kosh silo/stk.rs.

template < typename TElem>
class Stk
{
private:
    stalks::Atm< uint32_t>* _Size{nullptr};
    Arr< TElem>             _Arr{};

public:
    constexpr Stk( void) noexcept = default;

    constexpr Stk( stalks::Atm< uint32_t>* sizePtr, Arr< TElem> arr) noexcept
        : _Size( sizePtr),
          _Arr( arr)
    {
    }

    static constexpr Stk Create( stalks::Atm< uint32_t>* sizePtr, Arr< TElem> arr) noexcept
    {
        return { sizePtr, arr };
    }

    uint32_t Size( void) const noexcept
    {
        return _Size ? _Size->Load( std::memory_order_acquire) : 0;
    }

    void SetSize( uint32_t size) noexcept
    {
        if ( _Size)
            _Size->Store( size, std::memory_order_release);
    }

    uint32_t SzVoid( void) const noexcept
    {
        const uint32_t  sz = Size();
        const uint32_t  cap = _Arr.Size();
        return ( cap > sz) ? ( cap - sz) : 0;
    }

    USeg USeg( void) const noexcept
    {
        return silo::USeg( Size());
    }

    Arr< TElem> ArrView( void) const noexcept
    {
        return _Arr.RSnip( SzVoid());
    }

    bool Pop( TElem& val) const
    {
        if ( !_Size)
            return false;

        uint32_t        sz = Size();
        if ( sz == 0 || !_Size->CompareExchange( sz, sz - 1, std::memory_order_acquire, std::memory_order_relaxed))
            return false;

        _Arr.SwapAt( sz - 1, val);
        return true;
    }

    bool PushX( TElem& val) const
    {
        if ( !_Size)
            return false;

        uint32_t        sz = Size();
        if ( sz >= _Arr.Size())
            return false;

        _Arr.SwapAt( sz, val);
        if ( !_Size->CompareExchange( sz, sz + 1, std::memory_order_release, std::memory_order_relaxed)) {
            _Arr.SwapAt( sz, val);
            return false;
        }
        return true;
    }

    bool Push( TElem val) const
    {
        TElem           tmp = std::move( val);
        return PushX( tmp);
    }

    uint32_t Import( const Stk< TElem>& other, uint32_t maxMov = UINT32_MAX) const
        requires std::is_trivially_copyable_v< TElem>
    {
        if ( !_Size || !other._Size)
            return 0;

        uint32_t        szAlloc = 0;
        uint32_t        oldSz = 0;
        while ( true) {
            uint32_t        sz = Size();
            const uint32_t  szCacheVoid = ( _Arr.Size() > sz) ? ( _Arr.Size() - sz) : 0;
            const uint32_t  otherSz = other.Size();
            szAlloc = ( szCacheVoid < otherSz) ? szCacheVoid : otherSz;
            if ( szAlloc > maxMov)
                szAlloc = maxMov;

            if ( szAlloc == 0)
                return 0;

            if ( _Size->CompareExchange( sz, sz + szAlloc, std::memory_order_acq_rel, std::memory_order_acquire)) {
                oldSz = sz;
                break;
            }
        }
        const uint32_t  otherOldSz = other._Size->FetchSub( szAlloc, std::memory_order_acq_rel);
        const uint32_t  stkSz = otherOldSz - szAlloc;
        _Arr.SwapFrom( oldSz, other._Arr, stkSz, szAlloc);
        return szAlloc;
    }

    uint32_t Export( const Stk< TElem>& other, uint32_t maxMov = UINT32_MAX) const
        requires std::is_trivially_copyable_v< TElem>
    {
        if ( !_Size || !other._Size)
            return 0;

        uint32_t        szAlloc = 0;
        uint32_t        oldSz = 0;
        while ( true) {
            const uint32_t  szStk = other.Size();
            const uint32_t  szStkVoid = ( other._Arr.Size() > szStk) ? ( other._Arr.Size() - szStk) : 0;
            uint32_t        sz = Size();
            szAlloc = ( szStkVoid < sz) ? szStkVoid : sz;
            if ( szAlloc > maxMov)
                szAlloc = maxMov;

            if ( szAlloc == 0)
                return 0;

            if ( _Size->CompareExchange( sz, sz - szAlloc, std::memory_order_acq_rel, std::memory_order_acquire)) {
                oldSz = sz;
                break;
            }
        }
        const uint32_t  szStk = other._Size->FetchAdd( szAlloc, std::memory_order_acq_rel);
        other._Arr.SwapFrom( szStk, _Arr, oldSz - szAlloc, szAlloc);
        return szAlloc;
    }
};

} // namespace trellis::silo
