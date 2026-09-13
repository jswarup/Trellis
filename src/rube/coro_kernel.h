// coro_kernel.h ---------------------------------------------------------------------------------------------------
#pragma once

#include "rube/port.h"
#include "rube/reg.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/stash.h"

#include <algorithm>
#include <coroutine>
#include <cstdint>
#include <functional>
#include <memory>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::rube {

//-------------------------------------------------------------------------------------------------

constexpr uint32_t CORO_MAX_PORTS = 16;

//-------------------------------------------------------------------------------------------------
// Fixed-capacity port values array (up to 16 Reg values) for zero-allocation coroutine exchange.

struct CoroPorts
{
    Reg         _Vals[CORO_MAX_PORTS]{};
    uint32_t    _Len{0};

    constexpr CoroPorts( void) noexcept = default;

    constexpr explicit CoroPorts( Reg reg) noexcept
        : _Len{1}
    {
        _Vals[0] = reg;
    }

    constexpr CoroPorts( Reg r1, Reg r2) noexcept
        : _Len{2}
    {
        _Vals[0] = r1;
        _Vals[1] = r2;
    }

    static constexpr CoroPorts Empty( void) noexcept
    {
        return CoroPorts{};
    }

    static constexpr CoroPorts Single( Reg reg) noexcept
    {
        return CoroPorts{reg};
    }

    static constexpr CoroPorts Pair( Reg r1, Reg r2) noexcept
    {
        return CoroPorts{r1, r2};
    }

    static CoroPorts FromSlice( const Reg* slice, size_t count) noexcept
    {
        CoroPorts ports;
        const uint32_t limit = static_cast< uint32_t>( ( std::min)( count, static_cast< size_t>( CORO_MAX_PORTS)));
        for ( uint32_t i = 0; i < limit; ++i) {
            ports._Vals[i] = slice[i];
        }
        ports._Len = limit;
        return ports;
    }

    static CoroPorts FromArr( silo::Arr< const Reg> arr) noexcept
    {
        CoroPorts ports;
        const uint32_t limit = ( std::min)( arr.Size(), CORO_MAX_PORTS);
        for ( uint32_t i = 0; i < limit; ++i) {
            ports._Vals[i] = arr[i];
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

    Reg Get( uint32_t idx) const noexcept
    {
        return ( idx < _Len) ? _Vals[idx] : Reg::Unknown();
    }

    void Set( uint32_t idx, Reg val) noexcept
    {
        if ( idx < _Len) {
            _Vals[idx] = val;
        }
    }

    void Push( Reg val) noexcept
    {
        if ( _Len < CORO_MAX_PORTS) {
            _Vals[_Len++] = val;
        }
    }

    constexpr Reg operator[]( uint32_t idx) const noexcept
    {
        return _Vals[idx];
    }

    constexpr Reg& operator[]( uint32_t idx) noexcept
    {
        return _Vals[idx];
    }

    silo::Arr< const Reg> AsArr( void) const noexcept
    {
        return silo::Arr< const Reg>( _Vals, _Len);
    }
};

//-------------------------------------------------------------------------------------------------

enum class CoroResKind
{
    Yield,
    Done,
};

struct CoroRes
{
    CoroResKind _Kind{CoroResKind::Done};
    CoroPorts   _Ports{};

    static constexpr CoroRes Yield( CoroPorts ports) noexcept
    {
        return CoroRes{CoroResKind::Yield, ports};
    }

    static constexpr CoroRes Done( void) noexcept
    {
        return CoroRes{CoroResKind::Done, CoroPorts{}};
    }

    constexpr bool IsYield( void) const noexcept
    {
        return _Kind == CoroResKind::Yield;
    }

    constexpr bool IsDone( void) const noexcept
    {
        return _Kind == CoroResKind::Done;
    }

    constexpr CoroPorts Ports( void) const noexcept
    {
        return _Ports;
    }
};

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

