#ifndef TRELLIS_STALKS_ATM_H
#define TRELLIS_STALKS_ATM_H

//-------------------------------------------------------------------------------------------------

#include <atomic>
#include <cstdint>
#include <utility>

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

    T Get( void) const noexcept
    {
        return Load( std::memory_order_seq_cst);
    }

    void Set( T val) noexcept
    {
        Store( val, std::memory_order_seq_cst);
    }

    T Exchange( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.exchange( val, order);
    }

    T FetchAdd( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.fetch_add( val, order);
    }

    T Add( T val) noexcept
    {
        return FetchAdd( val, std::memory_order_seq_cst);
    }

    T FetchSub( T val, std::memory_order order = std::memory_order_seq_cst) noexcept
    {
        return _Val.fetch_sub( val, order);
    }

    T Sub( T val) noexcept
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

} // namespace trellis::stalks

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_STALKS_ATM_H

