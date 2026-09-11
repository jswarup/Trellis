#ifndef TRELLIS_STALKS_WORK_H
#define TRELLIS_STALKS_WORK_H

//-------------------------------------------------------------------------------------------------

#include <concepts>
#include <cstdint>
#include <type_traits>
#include <utility>

//-------------------------------------------------------------------------------------------------

namespace trellis::stalks {

class IWorker;

//-------------------------------------------------------------------------------------------------
// JobFn — function pointer type executing type-erased work on an IWorker.

using JobFn = void ( *)( void* data, IWorker* worker);

//-------------------------------------------------------------------------------------------------
// WorkPtr — 16-byte type-erased job pointer.

struct WorkPtr
{
    void*               _Data{nullptr};
    JobFn               _Func{nullptr};

    constexpr WorkPtr( void) noexcept
        : _Data( nullptr),
          _Func( []( void*, IWorker*) noexcept {})
    {
    }

    constexpr WorkPtr( void* data, JobFn func) noexcept
        : _Data( data),
          _Func( func)
    {
    }

    static constexpr WorkPtr Null( void) noexcept
    {
        return { nullptr, []( void*, IWorker*) noexcept {} };
    }

    static inline WorkPtr Dummy( void) noexcept
    {
        return { reinterpret_cast< void*>( 1), []( void*, IWorker*) noexcept {} };
    }

    constexpr bool IsNull( void) const noexcept
    {
        return _Data == nullptr;
    }

    void DoWork( IWorker* worker) const
    {
        if ( _Func) {
            _Func( _Data, worker);
        }
    }

template < typename TCallable>
        requires ( !std::is_same_v< std::decay_t< TCallable>, WorkPtr> && std::is_invocable_v< TCallable, IWorker*>)
    static WorkPtr FromLambda( TCallable&& callable)
    {
        using Decayed = std::decay_t< TCallable>;
        Decayed* storage = new Decayed( std::forward< TCallable>( callable));
        return WorkPtr(
            static_cast< void*>( storage),
            []( void* data, IWorker* worker) {
                Decayed* fn = static_cast< Decayed*>( data);
                ( *fn)( worker);
                delete fn;
            }
        );
    }

    static WorkPtr FromFn( void ( *fn)( IWorker*)) noexcept
    {
        return WorkPtr(
            reinterpret_cast< void*>( fn),
            []( void* data, IWorker* worker) {
                auto func = reinterpret_cast< void ( *)( IWorker*)>( data);
                if ( func) {
                    func( worker);
                }
            }
        );
    }
};

static_assert( sizeof( WorkPtr) == 16);

//-------------------------------------------------------------------------------------------------
// IWorker — abstract interface for worker contexts capable of receiving and scheduling jobs.

class IWorker
{
public:
    virtual ~IWorker( void) = default;

    virtual void PostJob( WorkPtr job) = 0;

    virtual const void* AsRawWorker( void) const noexcept
    {
        return nullptr;
    }

template < typename TCallable>
        requires std::is_invocable_v< TCallable, IWorker*>
    void Post( TCallable&& callable)
    {
        PostJob( WorkPtr::FromLambda( std::forward< TCallable>( callable)));
    }
};

//-------------------------------------------------------------------------------------------------
// Worker — immediate, sequential single-threaded worker executor.

class Worker : public IWorker
{
public:
    constexpr Worker( void) noexcept = default;

    static Worker New( void) noexcept
    {
        return {};
    }

    void PostJob( WorkPtr job) override
    {
        if ( !job.IsNull()) {
            job.DoWork( this);
        }
    }
};

} // namespace trellis::stalks

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_STALKS_WORK_H

