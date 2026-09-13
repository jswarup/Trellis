// atm.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include <atomic>
#include <cstdint>
#include <utility>
#include <thread>
#if defined(__x86_64__) || defined(_M_X64)
#include <immintrin.h>
#endif

//-------------------------------------------------------------------------------------------------

namespace trellis::stalks {

//-------------------------------------------------------------------------------------------------
// Atm — lightweight wrapper encapsulating standard atomic operations.
// Modeled directly from Kosh stalks/atm.rs.

template < typename T>
class Atm
{
private:
    std::atomic< T>     _Val{};

public:
    constexpr Atm( void) noexcept = default;

    constexpr explicit Atm( T val) noexcept
        : _Val( val)
    {
    }

    T Load( std::memory_order order = std::memory_order_seq_cst) const noexcept
    {
        return _Val.load( order);
    }

    void Store( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        _Val.store( val, order);
    }

    T   Get( void) const noexcept
    {
        return Load( std::memory_order_seq_cst);
    }

    void Set( T val) noexcept
    {
        Store( val, std::memory_order_seq_cst);
    }

    T   Exchange( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.exchange( val, order);
    }

    T   FetchAdd( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.fetch_add( val, order);
    }

    T   Add( T val) noexcept
    {
        return FetchAdd( val, std::memory_order_seq_cst);
    }

    T   FetchSub( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.fetch_sub( val, order);
    }

    T   Sub( T val) noexcept
    {
        return FetchSub( val, std::memory_order_seq_cst);
    }

    bool CompareExchange( T& expected, T desired, std::memory_order success = std::memory_order_seq_cst, std::memory_order failure = std::memory_order_seq_cst) noexcept
    {
        return _Val.compare_exchange_strong( expected, desired, success, failure);
    }

    bool CompareExchangeWeak( T& expected, T desired, std::memory_order success = std::memory_order_seq_cst, std::memory_order failure = std::memory_order_seq_cst) noexcept
    {
        return _Val.compare_exchange_weak( expected, desired, success, failure);
    }

    operator T( void) const noexcept
    {
        return Load();
    }

    T operator++( void) noexcept
    {
        return FetchAdd( 1) + 1;
    }

    T operator++( int) noexcept
    {
        return FetchAdd( 1);
    }

    T operator--( void) noexcept
    {
        return FetchSub( 1) - 1;
    }

    T operator--( int) noexcept
    {
        return FetchSub( 1);
    }

    T operator+=( T val) noexcept
    {
        return FetchAdd( val) + val;
    }

    T operator-=( T val) noexcept
    {
        return FetchSub( val) - val;
    }

    std::atomic< T>& Atomic( void) noexcept
    {
        return _Val;
    }

    const std::atomic< T>& Atomic( void) const noexcept
    {
        return _Val;
    }

    operator std::atomic< T>&( void) noexcept
    {
        return _Val;
    }

    operator const std::atomic< T>&( void) const noexcept
    {
        return _Val;
    }
};

//-------------------------------------------------------------------------------------------------
// SpinLockGuard — RAII scoped guard for Spinlock.

class Spinlock;

class SpinLockGuard
{
private:
    const Spinlock*     _Lock{nullptr};

public:
    explicit SpinLockGuard( const Spinlock* lock) noexcept;
    ~SpinLockGuard( void) noexcept;

    SpinLockGuard( const SpinLockGuard&) = delete;
    SpinLockGuard& operator=( const SpinLockGuard&) = delete;

    SpinLockGuard( SpinLockGuard&& o) noexcept
        : _Lock( std::exchange( o._Lock, nullptr))
    {
    }
};

//-------------------------------------------------------------------------------------------------
// Spinlock — low-latency atomic spinlock.

class Spinlock
{
private:
    mutable Atm< bool> _Locked{false};

public:
    constexpr Spinlock( void) noexcept = default;

    void    Acquire( void) const noexcept
    {
        while ( true)
        {
            if ( !_Locked.Exchange( true, std::memory_order_acquire))
                return;

            while ( _Locked.Load( std::memory_order_relaxed)) {
#if defined(__x86_64__) || defined(_M_X64)
                _mm_pause();
#else
                std::this_thread::yield();
#endif
            }
        }
    }

    void Release( void) const noexcept
    {
        _Locked.Store( false, std::memory_order_release);
    }

    [[nodiscard]] SpinLockGuard Lock( void) const noexcept
    {
        Acquire();
        return SpinLockGuard( this);
    }

    void lock( void) const noexcept
    {
        Acquire();
    }

    void unlock( void) const noexcept
    {
        Release();
    }
};

//-------------------------------------------------------------------------------------------------

inline SpinLockGuard::SpinLockGuard( const Spinlock* lock) noexcept
    : _Lock( lock)
{
}

inline SpinLockGuard::~SpinLockGuard( void) noexcept
{
    if ( _Lock)
        _Lock->Release();
}

//-------------------------------------------------------------------------------------------------
} // namespace trellis::stalks
