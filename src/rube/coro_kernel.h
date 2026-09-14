// coro_kernel.h ---------------------------------------------------------------------------------------------------
#pragma once

#include "rube/port.h"
#include "rube/trigger.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/stash.h"

#include <algorithm>
#include <coroutine>
#include <cstdint>
#include <functional>
#include <memory>
#include <type_traits>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------

constexpr uint32_t CORO_MAX_PORTS = 64;

//-------------------------------------------------------------------------------------------------
// Fixed-capacity port values array (up to 16 values) for zero-allocation coroutine exchange.

template < typename T = uint64_t>
struct CoroPortsT
{
    T           _Vals[CORO_MAX_PORTS]{};
    uint32_t    _Len{0};

    constexpr CoroPortsT( void) noexcept = default;

    template < typename U>
    constexpr explicit CoroPortsT( U val) noexcept
        : _Len{1}
    {
        if constexpr ( std::is_same_v< U, bool>) {
            _Vals[0] = val ? static_cast< T>( 1) : static_cast< T>( 0);
        } else {
            _Vals[0] = static_cast< T>( val);
        }
    }

    template < typename U1, typename U2>
    constexpr CoroPortsT( U1 v1, U2 v2) noexcept
        : _Len{2}
    {
        if constexpr ( std::is_same_v< U1, bool>) {
            _Vals[0] = v1 ? static_cast< T>( 1) : static_cast< T>( 0);
        } else {
            _Vals[0] = static_cast< T>( v1);
        }
        if constexpr ( std::is_same_v< U2, bool>) {
            _Vals[1] = v2 ? static_cast< T>( 1) : static_cast< T>( 0);
        } else {
            _Vals[1] = static_cast< T>( v2);
        }
    }

    static constexpr CoroPortsT Empty( void) noexcept
    {
        return CoroPortsT{};
    }

    template < typename U>
    static constexpr CoroPortsT Single( U val) noexcept
    {
        return CoroPortsT{val};
    }

    template < typename U1, typename U2>
    static constexpr CoroPortsT Pair( U1 v1, U2 v2) noexcept
    {
        return CoroPortsT{v1, v2};
    }

    template < typename U>
    static CoroPortsT FromSlice( const U* slice, size_t count) noexcept
    {
        CoroPortsT ports;
        const uint32_t limit = static_cast< uint32_t>( ( std::min)( count, static_cast< size_t>( CORO_MAX_PORTS)));
        for ( uint32_t i = 0; i < limit; ++i) {
            ports._Vals[i] = static_cast< T>( slice[i]);
        }
        ports._Len = limit;
        return ports;
    }

    template < typename U>
    static CoroPortsT FromArr( silo::Arr< const U> arr) noexcept
    {
        CoroPortsT ports;
        const uint32_t limit = ( std::min)( arr.Size(), CORO_MAX_PORTS);
        for ( uint32_t i = 0; i < limit; ++i) {
            ports._Vals[i] = static_cast< T>( arr[i]);
        }
        ports._Len = limit;
        return ports;
    }

    constexpr uint32_t Len( void) const noexcept
    {
        return _Len;
    }

    constexpr bool IsEmpty( void) const noexcept
    {
        return _Len == 0;
    }

    template < typename U = T>
    U Get( uint32_t idx) const noexcept
    {
        if ( idx >= _Len) return U{0};
        if constexpr ( std::is_same_v< U, bool>) {
            return ( _Vals[idx] & 1) != 0;
        } else {
            return static_cast< U>( _Vals[idx]);
        }
    }

    template < typename U>
    void Set( uint32_t idx, U val) noexcept
    {
        if ( idx < _Len) {
            if constexpr ( std::is_same_v< U, bool>) {
                _Vals[idx] = val ? static_cast< T>( 1) : static_cast< T>( 0);
            } else {
                _Vals[idx] = static_cast< T>( val);
            }
        }
    }

    template < typename U>
    void Push( U val) noexcept
    {
        if ( _Len < CORO_MAX_PORTS) {
            if constexpr ( std::is_same_v< U, bool>) {
                _Vals[_Len++] = val ? static_cast< T>( 1) : static_cast< T>( 0);
            } else {
                _Vals[_Len++] = static_cast< T>( val);
            }
        }
    }

    constexpr T operator[]( uint32_t idx) const noexcept
    {
        return _Vals[idx];
    }

    constexpr T& operator[]( uint32_t idx) noexcept
    {
        return _Vals[idx];
    }

    silo::Arr< const T> AsArr( void) const noexcept
    {
        return silo::Arr< const T>( _Vals, _Len);
    }
};

using CoroPorts = CoroPortsT< uint64_t>;

//-------------------------------------------------------------------------------------------------

enum class CoroResKind
{
    Yield,
    Done,
};

template < typename T = uint64_t>
struct CoroResT
{
    CoroResKind     _Kind{CoroResKind::Done};
    CoroPortsT< T>  _Ports{};

    static constexpr CoroResT Yield( CoroPortsT< T> ports) noexcept
    {
        return CoroResT{CoroResKind::Yield, ports};
    }

    static constexpr CoroResT Done( void) noexcept
    {
        return CoroResT{CoroResKind::Done, CoroPortsT< T>{}};
    }

    constexpr bool IsYield( void) const noexcept
    {
        return _Kind == CoroResKind::Yield;
    }

    constexpr bool IsDone( void) const noexcept
    {
        return _Kind == CoroResKind::Done;
    }

    constexpr CoroPortsT< T> Ports( void) const noexcept
    {
        return _Ports;
    }
};

using CoroRes = CoroResT< uint64_t>;

//-------------------------------------------------------------------------------------------------
// Sentinel for co_await CoroIn{} to retrieve input ports at the start of coroutine execution.

struct CoroIn {};

//-------------------------------------------------------------------------------------------------
// C++20 stackless coroutine task representing a single coroutine module instance.

class CoroTask
{
public:
    struct promise_type
    {
        CoroPorts _InPorts{};
        CoroPorts _OutPorts{};
        bool      _Done{false};

        CoroTask get_return_object( void) noexcept
        {
            return CoroTask{std::coroutine_handle< promise_type>::from_promise( *this)};
        }

        std::suspend_always initial_suspend( void) noexcept
        {
            return {};
        }

        std::suspend_always final_suspend( void) noexcept
        {
            return {};
        }

        void return_void( void) noexcept
        {
            _Done = true;
        }

        void unhandled_exception( void)
        {
            std::terminate();
        }

        auto await_transform( CoroIn) noexcept
        {
            struct InAwaiter
            {
                CoroPorts _Ports;
                bool await_ready( void) const noexcept { return true; }
                void await_suspend( std::coroutine_handle<>) const noexcept {}
                CoroPorts await_resume( void) const noexcept { return _Ports; }
            };
            return InAwaiter{_InPorts};
        }

        auto yield_value( CoroPorts outPorts) noexcept
        {
            _OutPorts = outPorts;
            struct YieldAwaiter
            {
                promise_type* _Promise;
                bool await_ready( void) const noexcept { return false; }
                void await_suspend( std::coroutine_handle<>) const noexcept {}
                CoroPorts await_resume( void) const noexcept
                {
                    return _Promise->_InPorts;
                }
            };
            return YieldAwaiter{this};
        }

    template < typename Awaiter>
        decltype( auto) await_transform( Awaiter&& awaiter) noexcept
        {
            return std::forward< Awaiter>( awaiter);
        }
    };

private:
    std::coroutine_handle< promise_type> _Handle{nullptr};

public:
    constexpr CoroTask( void) noexcept = default;

    explicit CoroTask( std::coroutine_handle< promise_type> handle) noexcept
        : _Handle{handle}
    {
    }

    ~CoroTask( void)
    {
        if ( _Handle) {
            _Handle.destroy();
            _Handle = nullptr;
        }
    }

    CoroTask( const CoroTask&) = delete;
    CoroTask& operator=( const CoroTask&) = delete;

    CoroTask( CoroTask&& other) noexcept
        : _Handle{other._Handle}
    {
        other._Handle = nullptr;
    }

    CoroTask& operator=( CoroTask&& other) noexcept
    {
        if ( this != &other) {
            if ( _Handle) {
                _Handle.destroy();
            }
            _Handle = other._Handle;
            other._Handle = nullptr;
        }
        return *this;
    }

    bool IsDone( void) const noexcept
    {
        return !_Handle || _Handle.done() || _Handle.promise()._Done;
    }

    CoroRes Resume( CoroPorts inPorts)
    {
        if ( IsDone()) {
            return CoroRes::Done();
        }
        _Handle.promise()._InPorts = inPorts;
        _Handle.resume();
        if ( _Handle.done() || _Handle.promise()._Done) {
            return CoroRes::Done();
        }
        return CoroRes::Yield( _Handle.promise()._OutPorts);
    }
};

//-------------------------------------------------------------------------------------------------

using CoroKernelFactory = std::function< CoroTask( void)>;

//-------------------------------------------------------------------------------------------------
// Cell container holding a CoroTask instance with copy-constructible wrapper for buffer storage.

class CoroCell
{
    mutable CoroTask _Task{};

public:
    CoroCell( void) noexcept = default;

    explicit CoroCell( CoroTask task) noexcept
        : _Task{std::move( task)}
    {
    }

    CoroCell( CoroCell&& other) noexcept
        : _Task{std::move( other._Task)}
    {
    }

    CoroCell& operator=( CoroCell&& other) noexcept
    {
        if ( this != &other) {
            _Task = std::move( other._Task);
        }
        return *this;
    }

    CoroCell( const CoroCell& other) noexcept
        : _Task{std::move( other._Task)}
    {
    }

    CoroCell& operator=( const CoroCell& other) noexcept
    {
        if ( this != &other) {
            _Task = std::move( other._Task);
        }
        return *this;
    }

    CoroRes Resume( CoroPorts inPorts) const
    {
        return _Task.Resume( inPorts);
    }

    bool IsDone( void) const noexcept
    {
        return _Task.IsDone();
    }
};

//-------------------------------------------------------------------------------------------------
// Compiled warp executing a batch of homogeneous coroutine module instances.

struct CoroWarp
{
    uint32_t                            _ModStart{0};
    uint32_t                            _Count{0};
    silo::Buff< CoroCell>               _Instances{};
    silo::Buff< silo::Buff< TriggerId>> _InTriggers{};
    silo::Buff< silo::Buff< TriggerId>> _OutTriggers{};

    constexpr CoroWarp( void) noexcept = default;

    CoroWarp(
        uint32_t modStart,
        uint32_t count,
        silo::Buff< CoroCell> instances,
        silo::Buff< silo::Buff< TriggerId>> inTriggers,
        silo::Buff< silo::Buff< TriggerId>> outTriggers) noexcept
        : _ModStart{modStart},
          _Count{count},
          _Instances{std::move( instances)},
          _InTriggers{std::move( inTriggers)},
          _OutTriggers{std::move( outTriggers)}
    {
    }
};

} // namespace trellis::rube

