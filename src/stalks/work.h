// work.h ---------------------------------------------------------------------------------------------------------
#pragma once

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
// IWorker — zero-virtual worker context capable of receiving and scheduling jobs.

using PostJobFn = void (*)( void* self, WorkPtr job);

class IWorker
{
protected:
    PostJobFn   _PostJob{nullptr};

public:
    constexpr IWorker( void) noexcept = default;

    constexpr explicit IWorker( PostJobFn postJob) noexcept
        : _PostJob( postJob)
    {
    }

    void PostJob( WorkPtr job)
    {
        if ( _PostJob) {
            _PostJob( this, job);
        }
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
    constexpr Worker( void) noexcept
        : IWorker( []( void* self, WorkPtr job) {
            if ( !job.IsNull()) {
                job.DoWork( static_cast< Worker*>( self));
            }
        })
    {
    }

    void PostJob( WorkPtr job)
    {
        if ( !job.IsNull()) {
            job.DoWork( this);
        }
    }
};

} // namespace trellis::stalks
