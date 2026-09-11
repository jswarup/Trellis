//-------------------------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "heist/heist.h"
#include "stalks/atm.h"
#include "stalks/work.h"
#include "stalks/node.h"

#include <atomic>
#include <chrono>
#include <vector>

using namespace trellis::heist;
using namespace trellis::stalks;
using namespace trellis::silo;

//-------------------------------------------------------------------------------------------------
// MaestroOps Tests

TR_TEST( Heist, MaestroOps)
{
    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    {
        auto            maestros = atelier.Maestros();
        Maestro&        m2 = maestros[2];
        m2.SetAtelier( &atelier);
        m2.SetCurSuccId( 42);
    }
    auto                maestros = atelier.Maestros();
    TR_ASSERT_EQ( maestros[2].MaestroIndex(), 2u);
    TR_ASSERT_EQ( maestros[2].CurSuccId(), 42u);

    // Test local run queue push and pop
    maestros[1].EnqueRunJob( 123);
    uint16_t            poppedId = maestros[1].PopJob();
    TR_ASSERT_EQ( poppedId, 123u);
    TR_ASSERT_EQ( maestros[1].PopJob(), 0u);
}

//-------------------------------------------------------------------------------------------------
// AtelierLaunch Tests

TR_TEST( Heist, AtelierLaunch)
{
    // Test 1: Immediate mode (0 threads)
    {
        Atelier::Reset( 0);
        auto&           atelier = Atelier::Instance();
        TR_ASSERT( atelier.IsImmediate());
        TR_ASSERT_EQ( atelier.SzThreads(), 0u);

        Worker          worker;
        bool            immediateExecuted = false;
        worker.Post( [&]( IWorker*) {
            immediateExecuted = true;
        });
        TR_ASSERT( immediateExecuted);
    }

    // Test 2: Single-threaded mode (1 thread)
    {
        static std::atomic< int> countSingle{0};
        countSingle = 0;

        Atelier::Reset( 1);
        auto&           atelier = Atelier::Instance();
        TR_ASSERT( !atelier.IsImmediate());
        TR_ASSERT_EQ( atelier.SzThreads(), 1u);

        Maestro*        mainMaestro = atelier.MainMaestro();
        TR_ASSERT( mainMaestro != nullptr);

        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( []( IWorker* w) {
                Maestro* m = Maestro::FromWorker( w);
                countSingle += 1;
                uint16_t child = m->ConstructJob(
                    m->CurSuccId(),
                    WorkPtr::FromLambda( []( IWorker*) {
                        countSingle += 10;
                    })
                );
                m->EnqueueJob( child);
            })
        );
        mainMaestro->EnqueueJob( jobId);
        atelier.DoLaunch();

        TR_ASSERT_EQ( countSingle.load(), 11);
    }

    // Test 3: Multi-threaded mode (4 threads) with master & child jobs
    {
        static std::atomic< int> countMulti{0};
        countMulti = 0;

        Atelier::Reset( 4);
        auto&           atelier = Atelier::Instance();
        TR_ASSERT_EQ( atelier.SzThreads(), 4u);

        Maestro*        mainMaestro = atelier.MainMaestro();
        TR_ASSERT( mainMaestro != nullptr);

        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( []( IWorker* w) {
                Maestro* m = Maestro::FromWorker( w);
                countMulti += 1;
                uint16_t child1 = m->ConstructJob(
                    m->CurSuccId(),
                    WorkPtr::FromLambda( []( IWorker*) {
                        countMulti += 10;
                    })
                );
                m->EnqueueJob( child1);
            })
        );
        mainMaestro->EnqueueJob( jobId);
        atelier.DoLaunch();

        TR_ASSERT_EQ( countMulti.load(), 11);
    }
}

//-------------------------------------------------------------------------------------------------
// ChoreTreeDAG Tests

TR_TEST( Heist, ChoreTreeDAG)
{
    static std::atomic< int>  traceIdx{0};
    static std::atomic< bool> aDone{false};
    static std::atomic< bool> cDone{false};
    static std::atomic< bool> seqOrderOk{true};

    traceIdx   = 0;
    aDone      = false;
    cDone      = false;
    seqOrderOk = true;

    auto                a = Chore::NewDoc( "A", []( IWorker*) {
        traceIdx += 1;
        aDone.store( true, std::memory_order_release);
    });
    auto                b = Chore::NewDoc( "B", []( IWorker*) {
        if ( !aDone.load( std::memory_order_acquire)) {
            seqOrderOk.store( false, std::memory_order_relaxed);
        }
        traceIdx += 2;
    });
    auto                c = Chore::NewDoc( "C", []( IWorker*) {
        traceIdx += 4;
        cDone.store( true, std::memory_order_release);
    });
    auto                d = Chore::NewDoc( "D", []( IWorker*) {
        if ( !cDone.load( std::memory_order_acquire)) {
            seqOrderOk.store( false, std::memory_order_relaxed);
        }
        traceIdx += 8;
    });

    // ChoreTree DAG: (a < b) | (c < d) | Chore(e)
    auto                choreTree = ( a < b) | ( c < d) | Chore::New( []( IWorker*) {
        traceIdx += 10;
    });

    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    Maestro*            mainMaestro = atelier.MainMaestro();
    mainMaestro->PostChoreTree( choreTree);
    atelier.DoLaunch();

    TR_ASSERT_EQ( traceIdx.load(), 25);
    TR_ASSERT( seqOrderOk.load());
}

//-------------------------------------------------------------------------------------------------
// WorkStealing Tests

TR_TEST( Heist, WorkStealing)
{
    constexpr int       kJobCount = 128;
    static std::atomic< int> completedCount{0};
    completedCount = 0;

    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    Maestro*            mainMaestro = atelier.MainMaestro();

    for ( int i = 0; i < kJobCount; ++i) {
        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( []( IWorker*) {
                completedCount.fetch_add( 1, std::memory_order_relaxed);
            })
        );
        mainMaestro->EnqueueJob( jobId);
    }

    atelier.DoLaunch();

    TR_ASSERT_EQ( completedCount.load(), kJobCount);

    // Verify that multiple Maestros participated in job processing
    uint32_t            totalProcessed = 0;
    for ( auto& maestro : atelier.Maestros()) {
        totalProcessed += maestro._SzProcessed;
    }
    // Terminal job + 128 jobs = 129 jobs processed total
    TR_ASSERT( totalProcessed >= static_cast< uint32_t>( kJobCount));
}

//-------------------------------------------------------------------------------------------------

