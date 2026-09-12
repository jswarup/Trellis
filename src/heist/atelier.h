#ifndef TRELLIS_HEIST_ATELIER_H
#define TRELLIS_HEIST_ATELIER_H

//-------------------------------------------------------------------------------------------------

#include "heist/choretree.h"
#include "heist/maestro.h"
#include "silo/buff.h"
#include "silo/stash.h"
#include "stalks/atm.h"
#include "stalks/work.h"

#include <cstdint>
#include <new>
#include <thread>
#include <vector>
#if defined(__x86_64__) || defined(_M_X64)
#include <immintrin.h>
#endif

//-------------------------------------------------------------------------------------------------

namespace trellis::heist {

//-------------------------------------------------------------------------------------------------
// Atelier — singleton work-stealing job pool, DAG dependency resolver, and multi-threaded orchestrator.
//
//   szThreads == 0  → Immediate mode: PostJob executes inline, DoLaunch is a no-op.
//   szThreads == 1  → Single-threaded: main thread runs as sole Maestro.
//   szThreads >= 2  → Multi-threaded: N Maestros with work-stealing.

class Atelier
{
    friend class Maestro;

    static constexpr uint32_t k_JobCapacity = 65536;

    uint32_t                                _SzThreads{0};
    mutable stalks::Atm< uint32_t>          _SzSchedJob{0};
    silo::Buff< Maestro>                    _Maestros{};
    silo::Buff< stalks::Atm< uint16_t>>     _SzPreds{};
    silo::Buff< uint16_t>                   _SuccIds{};
    stalks::Spinlock                        _FreeJobLock{};
    silo::Stash< uint16_t>                  _FreeJobStash{};
    silo::Buff< stalks::WorkPtr>            _JobBuff{};
    uint16_t                                _Terminal{0};

    //---------------------------------------------------------------------------------------------

    constexpr Atelier( void) noexcept = default;

    explicit Atelier( uint32_t szThreads)
        : _SzThreads( szThreads),
          _SzSchedJob( 0),
          _Maestros( szThreads == 0 ? 1 : szThreads, []( uint32_t i) { return Maestro::New( i); }),
          _SzPreds( k_JobCapacity, []( uint32_t) { return stalks::Atm< uint16_t>( 0); }),
          _SuccIds( k_JobCapacity, static_cast< uint16_t>( 0)),
          _FreeJobStash( k_JobCapacity, 0, static_cast< uint16_t>( 0)),
          _JobBuff( k_JobCapacity, stalks::WorkPtr::Null()),
          _Terminal( 0)
    {
        _FreeJobStash.DoIndexSetup();
        _Terminal = ConstructJob( 0, 0, stalks::WorkPtr::Dummy());
        _Maestros[0].SetCurSuccId( _Terminal);
    }

public:

    //---------------------------------------------------------------------------------------------
    // Singleton

    static Atelier& Instance( void) noexcept
    {
        alignas( Atelier) static uint8_t s_Storage[sizeof( Atelier)]{};
        static Atelier* s_Ptr = new ( s_Storage) Atelier();
        return *s_Ptr;
    }

    static void Boot( uint32_t szThreads)
    {
        auto& inst = Instance();
        inst.~Atelier();
        new ( &inst) Atelier( szThreads);
    }

    static void Reset( uint32_t szThreads)
    {
        auto& inst = Instance();
        inst.~Atelier();
        new ( &inst) Atelier( szThreads);
    }

    static uint32_t DefaultThreadCount( void) noexcept
    {
        uint32_t hw = std::thread::hardware_concurrency();
        return ( hw > 0) ? hw : 1;
    }

    //---------------------------------------------------------------------------------------------
    // Mode queries

    bool IsImmediate( void) const noexcept
    {
        return _SzThreads == 0;
    }

    uint32_t SzThreads( void) const noexcept
    {
        return _SzThreads;
    }

    //---------------------------------------------------------------------------------------------
    // Accessors

    Maestro* MainMaestro( void) noexcept
    {
        if ( _Maestros.Size() == 0) {
            return nullptr;
        }
        Maestro* m = &_Maestros[0];
        m->SetAtelier( this);
        return m;
    }

    uint16_t Terminal( void) const noexcept
    {
        return _Terminal;
    }

    silo::Arr< Maestro> Maestros( void) noexcept
    {
        return _Maestros;
    }

    uint16_t SuccId( uint16_t jobId) const noexcept
    {
        return _SuccIds[jobId];
    }

    stalks::Atm< uint16_t>* SzPred( uint16_t jobId) noexcept
    {
        return &_SzPreds[jobId];
    }

    //---------------------------------------------------------------------------------------------
    // Job lifecycle

    uint16_t AllocJob( uint32_t maestroIdx)
    {
        Maestro& maestro = _Maestros[maestroIdx];
        auto jobCacheStk = maestro.JobCacheStk();
        while ( true) {
            uint16_t jobId = 0;
            if ( jobCacheStk.Size() != 0 && jobCacheStk.Pop( jobId)) {
                return jobId;
            }
            if ( _FreeJobStash.Size() == 0) {
                std::this_thread::yield();
                continue;
            }
            auto guard = _FreeJobLock.Lock();
            _FreeJobStash.StkView().Export( jobCacheStk, UINT32_MAX);
        }
    }

    bool FreeJob( uint32_t maestroIdx, uint16_t jobId)
    {
        Maestro& maestro = _Maestros[maestroIdx];
        maestro.FlushTempQueue();
        auto jobCacheStk = maestro.JobCacheStk();
        while ( true) {
            if ( jobCacheStk.SzVoid() != 0 && jobCacheStk.PushX( jobId)) {
                return true;
            }
            auto guard = _FreeJobLock.Lock();
            _FreeJobStash.StkView().Import( jobCacheStk, UINT32_MAX);
        }
    }

    void SetSucc( uint16_t jobId, uint16_t succId)
    {
        _SuccIds.SetAt( jobId, succId);
        SzPred( succId)->Add( 1);
    }

    uint16_t ConstructJob( uint32_t maestroIdx, uint16_t succId, stalks::WorkPtr job)
    {
        uint16_t jobId = AllocJob( maestroIdx);
        if ( jobId == 0) {
            return jobId;
        }
        _JobBuff.SetAt( jobId, job);
        if ( succId != 0) {
            SetSucc( jobId, succId);
        }
        return jobId;
    }

    //---------------------------------------------------------------------------------------------
    // Work-stealing

    uint16_t GrabJob( uint32_t idx, uint32_t& stealSeed)
    {
        const uint32_t sz = _Maestros.Size();
        constexpr uint32_t knuthMultHash = 2654435761u;
        stealSeed = stealSeed * knuthMultHash + 1u;
        for ( uint32_t mIdx = 0; mIdx < sz; ++mIdx) {
            uint32_t maestroIdx = ( stealSeed + mIdx) % sz;
            if ( maestroIdx == idx) {
                continue;
            }
            uint16_t jobId = _Maestros[maestroIdx].PopJob();
            if ( jobId != 0) {
                return jobId;
            }
        }
        return 0;
    }

    //---------------------------------------------------------------------------------------------
    // Execution

    void ExecuteLoop( uint32_t maestroIdx)
    {
        Maestro& maestro = _Maestros[maestroIdx];
        maestro.SetAtelier( this);
        maestro.FlushTempQueue();
        uint16_t jobId = 0;
        uint32_t stealSeed = maestroIdx;

        while ( _SzSchedJob.Load( std::memory_order_acquire) != 0) {
            while ( jobId != 0) {
                maestro.SetCurSuccId( _SuccIds[jobId]);
                stalks::WorkPtr job = _JobBuff[jobId];
                job.DoWork( &maestro);

                _JobBuff.SetAt( jobId, stalks::WorkPtr::Null());
                maestro._SzProcessed += 1;

                FreeJob( maestroIdx, jobId);
                uint16_t succId = maestro.CurSuccId();
                if ( succId != 0) {
                    uint16_t szPred = SzPred( succId)->Sub( 1);
                    if ( szPred == 1) {
                        jobId = succId;
                        _SzSchedJob.Add( 1);
                    } else {
                        jobId = 0;
                    }
                } else {
                    jobId = 0;
                }
                _SzSchedJob.Sub( 1);
            }

            jobId = maestro.PopJob();
            if ( jobId == 0 && _SzThreads >= 2) {
                jobId = GrabJob( maestroIdx, stealSeed);
            }
            if ( jobId == 0) {
#if defined(__x86_64__) || defined(_M_X64)
                _mm_pause();
#endif
                std::this_thread::yield();
            }
        }
    }

    void DoLaunch( void)
    {
        if ( _SzThreads == 0) {
            return;
        }

        _Maestros[0].FlushTempQueue();

        if ( _SzThreads == 1) {
            ExecuteLoop( 0);
            return;
        }

        const uint32_t sz = _Maestros.Size();
        std::vector< std::jthread> threads;
        threads.reserve( sz > 0 ? sz - 1 : 0);

        for ( uint32_t i = 1; i < sz; ++i) {
            threads.emplace_back( [this, i]() {
                this->ExecuteLoop( i);
            });
        }
        ExecuteLoop( 0);
        for ( auto& t : threads) {
            if ( t.joinable()) {
                t.join();
            }
        }
    }
};

//-------------------------------------------------------------------------------------------------
// Inlined Maestro implementations requiring Atelier

inline uint16_t Maestro::ConstructJob( uint16_t succId, stalks::WorkPtr job)
{
    return _Atelier->ConstructJob( _Index, succId, job);
}

inline uint16_t Maestro::ConstructEnqueArr( uint16_t succId, silo::Buff< uint16_t> buff)
{
    return ConstructJob(
        succId,
        stalks::WorkPtr::FromLambda( [buff]( stalks::IWorker* worker) {
            Maestro* maestro = Maestro::FromWorker( worker);
            if ( maestro) {
                for ( uint16_t jobId : buff) {
                    maestro->EnqueueJob( jobId);
                }
            }
        })
    );
}

inline void Maestro::FlushTempQueue( void)
{
    auto arr = _TempQueue.StkView().ArrView();
    for ( uint16_t jobId : arr) {
        if ( jobId != 0) {
            _Atelier->_SzSchedJob.Add( 1);
            EnqueRunJob( jobId);
        }
    }
    _TempQueue.Clear();
}

inline void Maestro::PostJob( stalks::WorkPtr job)
{
    if ( _Atelier && _Atelier->IsImmediate()) {
        job.DoWork( this);
        return;
    }
    uint16_t succId = CurSuccId();
    uint16_t jobId = ConstructJob( succId, job);
    EnqueueJob( jobId);
}

template < typename TChoreNode>
inline void Maestro::PostChoreTree( const TChoreNode& node)
{
    silo::Stash< uint16_t> tails = silo::Stash< uint16_t>::New( 64, 0, static_cast< uint16_t>( 0));
    uint16_t head = PostChoreNode( node, this, tails);
    uint16_t succId = CurSuccId();
    uint16_t tail = 0;
    while ( tails.Pop( tail)) {
        _Atelier->SetSucc( tail, succId);
    }
    EnqueueJob( head);
}

//-------------------------------------------------------------------------------------------------
// Chore & BinNode Post implementations

inline uint16_t Chore::Post( Maestro* maestro, silo::Stash< uint16_t>& tails) const
{
    uint16_t jobId = maestro->ConstructJob( 0, stalks::WorkPtr::FromFn( _Closure));
    tails.Push( jobId);
    return jobId;
}

template < typename L, typename R, typename Op>
inline uint16_t PostChoreNode( const stalks::BinNode< L, R, Op>& node, Maestro* maestro, silo::Stash< uint16_t>& tails)
{
    if ( node._Op == stalks::BinOp::Bor) {
        silo::Stash< uint16_t> leftTails = silo::Stash< uint16_t>::New( 64, 0, static_cast< uint16_t>( 0));
        silo::Stash< uint16_t> rightTails = silo::Stash< uint16_t>::New( 64, 0, static_cast< uint16_t>( 0));
        uint16_t headL = PostChoreNode( node._Left, maestro, leftTails);
        uint16_t headR = PostChoreNode( node._Right, maestro, rightTails);

        uint16_t t = 0;
        while ( leftTails.Pop( t)) {
            tails.Push( t);
        }
        while ( rightTails.Pop( t)) {
            tails.Push( t);
        }

        silo::Buff< uint16_t> heads( 2, static_cast< uint16_t>( 0));
        heads[0] = headL;
        heads[1] = headR;
        return maestro->ConstructEnqueArr( 0, std::move( heads));
    } else if ( node._Op == stalks::BinOp::Less) {
        silo::Stash< uint16_t> leftTails = silo::Stash< uint16_t>::New( 64, 0, static_cast< uint16_t>( 0));
        uint16_t headL = PostChoreNode( node._Left, maestro, leftTails);
        uint16_t headR = PostChoreNode( node._Right, maestro, tails);

        uint16_t leftTail = 0;
        while ( leftTails.Pop( leftTail)) {
            maestro->AtelierRef()->SetSucc( leftTail, headR);
        }
        return headL;
    }
    return 0;
}

} // namespace trellis::heist

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_HEIST_ATELIER_H

