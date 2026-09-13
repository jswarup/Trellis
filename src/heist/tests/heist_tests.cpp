// heist_tests.cpp --------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "heist/heist.h"
#include "stalks/atm.h"
#include "stalks/work.h"
#include "stalks/node.h"

#include <algorithm>
#include <chrono>
#include <random>

using namespace trellis::heist;
using namespace trellis::stalks;
using namespace trellis::silo;

//-------------------------------------------------------------------------------------------------
// MaestroOps Tests

JEEVES_TEST( Heist, MaestroOps)
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
    JEEVES_ASSERT_EQ( maestros[2].MaestroIndex(), 2u);
    JEEVES_ASSERT_EQ( maestros[2].CurSuccId(), 42u);

    // Test local run queue push and pop
    maestros[1].EnqueRunJob( 123);
    uint16_t            poppedId = maestros[1].PopJob();
    JEEVES_ASSERT_EQ( poppedId, 123u);
    JEEVES_ASSERT_EQ( maestros[1].PopJob(), 0u);
}

//-------------------------------------------------------------------------------------------------
// AtelierLaunch Tests

JEEVES_TEST( Heist, AtelierLaunch)
{
    // Test 1: Immediate mode (0 threads)
    // In 0-thread mode, Atelier allocates a single Maestro acting as a pass-through Worker.
    // Jobs are executed immediately inline. DoLaunch is a no-op.
    {
        Atelier::Reset( 0);
        auto&           atelier = Atelier::Instance();
        JEEVES_ASSERT( atelier.IsImmediate());
        JEEVES_ASSERT_EQ( atelier.SzThreads(), 0u);

        Maestro*        mainMaestro = atelier.MainMaestro();
        JEEVES_ASSERT( mainMaestro != nullptr);

        bool            immediateExecuted = false;
        mainMaestro->Post( [&]( IWorker*) {
            immediateExecuted = true;
        });
        JEEVES_ASSERT( immediateExecuted);

        atelier.DoLaunch(); // Safe no-op
    }

    // Test 2: Single-threaded (1) and Multi-threaded (4) mode
    for ( uint32_t szThreads : { 1u, 4u }) {
        Atm< int>       count{0};

        Atelier::Reset( szThreads);
        auto&           atelier = Atelier::Instance();
        JEEVES_ASSERT( !atelier.IsImmediate());
        JEEVES_ASSERT_EQ( atelier.SzThreads(), szThreads);

        Maestro*        mainMaestro = atelier.MainMaestro();
        JEEVES_ASSERT( mainMaestro != nullptr);

        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( [&count]( IWorker* w) {
                Maestro* m = Maestro::FromWorker( w);
                count += 1;
                uint16_t child = m->ConstructJob(
                    m->CurSuccId(),
                    WorkPtr::FromLambda( [&count]( IWorker*) {
                        count += 10;
                    })
                );
                m->EnqueueJob( child);
            })
        );
        mainMaestro->EnqueueJob( jobId);
        atelier.DoLaunch();

        JEEVES_ASSERT_EQ( count.Load(), 11);
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Heist, AtelierResetAfterLaunch)
{
    for ( uint32_t iteration = 0; iteration < 8; ++iteration) {
        Atm< uint32_t> completed{0};

        Atelier::Reset( 4);
        auto&               atelier = Atelier::Instance();
        Maestro*            mainMaestro = atelier.MainMaestro();
        uint16_t            jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( [&completed]( IWorker*) {
                completed.FetchAdd( 1, std::memory_order_relaxed);
            })
        );
        mainMaestro->EnqueueJob( jobId);
        atelier.DoLaunch();

        JEEVES_ASSERT_EQ( completed.Load(), 1u);
        Atelier::Reset( 0);
        JEEVES_ASSERT( Atelier::Instance().IsImmediate());
    }
}

//-------------------------------------------------------------------------------------------------
// ChoreTreeDAG Tests

JEEVES_TEST( Heist, ChoreTreeDAG)
{
    static Atm< int>  traceIdx{0};
    static Atm< bool> aDone{false};
    static Atm< bool> cDone{false};
    static Atm< bool> seqOrderOk{true};

    traceIdx.Store( 0);
    aDone.Store( false);
    cDone.Store( false);
    seqOrderOk.Store( true);

    auto                a = Chore( "A", []( IWorker*) {
        traceIdx += 1;
        aDone.Store( true, std::memory_order_release);
    });
    auto                b = Chore( "B", []( IWorker*) {
        if ( !aDone.Load( std::memory_order_acquire)) {
            seqOrderOk.Store( false, std::memory_order_relaxed);
        }
        traceIdx += 2;
    });
    auto                c = Chore( "C", []( IWorker*) {
        traceIdx += 4;
        cDone.Store( true, std::memory_order_release);
    });
    auto                d = Chore( "D", []( IWorker*) {
        if ( !cDone.Load( std::memory_order_acquire)) {
            seqOrderOk.Store( false, std::memory_order_relaxed);
        }
        traceIdx += 8;
    });

    // ChoreTree DAG: (a < b) | (c < d) | Chore(e)
    auto                choreTree = ( a < b) | ( c < d) | Chore( []( IWorker*) {
        traceIdx += 10;
    });

    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    Maestro*            mainMaestro = atelier.MainMaestro();
    mainMaestro->PostChoreTree( choreTree);
    atelier.DoLaunch();

    JEEVES_ASSERT_EQ( traceIdx.Load(), 25);
    JEEVES_ASSERT( seqOrderOk.Load());
}

//-------------------------------------------------------------------------------------------------
// WorkStealing Tests

JEEVES_TEST( Heist, WorkStealing)
{
    constexpr int       kJobCount = 256;
    Atm< int>           completedCount{0};

    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    Maestro*            mainMaestro = atelier.MainMaestro();

    for ( int i = 0; i < kJobCount; ++i) {
        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( [&completedCount]( IWorker*) {
                std::this_thread::yield(); // Induce slight delay to encourage stealing
                completedCount.FetchAdd( 1, std::memory_order_relaxed);
            })
        );
        mainMaestro->EnqueueJob( jobId);
    }

    atelier.DoLaunch();

    if ( ctx->_Verbosity > 0) {
        std::cout << "         [WorkStealing] Chores executed:\n";
        auto                maestros = atelier.Maestros();
        for ( uint32_t i = 0; i < maestros.Size(); ++i) {
            std::cout << "           Thread " << i << ": " << maestros[i]._SzProcessed << "\n";
        }
    }

    JEEVES_ASSERT_EQ( completedCount.Load(), kJobCount);

    // Verify that other Maestros ACTUALLY stole work
    uint32_t            stolenJobs = 0;
    auto                maestros = atelier.Maestros();
    for ( uint32_t i = 1; i < maestros.Size(); ++i) {
        stolenJobs += maestros[i]._SzProcessed;
    }

    // At least 1 job must have been processed by a worker other than mainMaestro
    JEEVES_ASSERT( stolenJobs > 0);

    uint32_t            totalProcessed = stolenJobs + maestros[0]._SzProcessed;
    JEEVES_ASSERT( totalProcessed >= static_cast< uint32_t>( kJobCount));
}

//-------------------------------------------------------------------------------------------------
// DoQSort Tests

JEEVES_TEST( Heist, DoQSort)
{
    constexpr uint32_t  kArraySize = 4096;
    std::mt19937        rng( 1337);
    std::uniform_real_distribution< float> dist( -1000.0f, 1000.0f);

    Buff< float> initialData( kArraySize, [&]( uint32_t) {
        return dist( rng);
    });

    Buff< float> expected = initialData;
    std::sort( expected.begin(), expected.end());

    for ( uint32_t szThreads : { 0u, 1u, 5u }) {
        Buff< float>    data = initialData;
        USeg            seg = USeg( data.Size());

        Atelier::Reset( szThreads);
        auto&           atelier = Atelier::Instance();
        Maestro*        mainMaestro = atelier.MainMaestro();
        JEEVES_ASSERT( mainMaestro != nullptr);

        mainMaestro->Post( [&]( IWorker* w) {
            seg.DoQSort(
                w,
                [&]( uint32_t i, uint32_t j) { return data[i] < data[j]; },
                [&]( uint32_t i, uint32_t j) { std::swap( data[i], data[j]); }
            );
        });
        atelier.DoLaunch();

        if ( ctx->_Verbosity > 0) {
            std::cout << "         [Threads: " << szThreads << "] Chores executed:\n";
            auto            maestros = atelier.Maestros();
            for ( uint32_t i = 0; i < maestros.Size(); ++i) {
                std::cout << "           Thread " << i << ": " << maestros[i]._SzProcessed << "\n";
            }
        }

        JEEVES_ASSERT( std::is_sorted( data.begin(), data.end()));
        JEEVES_ASSERT_EQ( data.First(), expected.First());
        JEEVES_ASSERT_EQ( data.Last(), expected.Last());
        JEEVES_ASSERT_EQ( data, expected);
    }
}

//-------------------------------------------------------------------------------------------------

