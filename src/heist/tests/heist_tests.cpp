// heist_tests.cpp --------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "heist/heist.h"
#include "stalks/atm.h"
#include "stalks/work.h"
#include "stalks/node.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <random>
#include <vector>

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
        std::atomic< int> count{0};

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

        JEEVES_ASSERT_EQ( count.load(), 11);
    }
}

//-------------------------------------------------------------------------------------------------
// ChoreTreeDAG Tests

JEEVES_TEST( Heist, ChoreTreeDAG)
{
    static std::atomic< int>  traceIdx{0};
    static std::atomic< bool> aDone{false};
    static std::atomic< bool> cDone{false};
    static std::atomic< bool> seqOrderOk{true};

    traceIdx   = 0;
    aDone      = false;
    cDone      = false;
    seqOrderOk = true;

    auto                a = Chore( "A", []( IWorker*) {
        traceIdx += 1;
        aDone.store( true, std::memory_order_release);
    });
    auto                b = Chore( "B", []( IWorker*) {
        if ( !aDone.load( std::memory_order_acquire)) {
            seqOrderOk.store( false, std::memory_order_relaxed);
        }
        traceIdx += 2;
    });
    auto                c = Chore( "C", []( IWorker*) {
        traceIdx += 4;
        cDone.store( true, std::memory_order_release);
    });
    auto                d = Chore( "D", []( IWorker*) {
        if ( !cDone.load( std::memory_order_acquire)) {
            seqOrderOk.store( false, std::memory_order_relaxed);
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

    JEEVES_ASSERT_EQ( traceIdx.load(), 25);
    JEEVES_ASSERT( seqOrderOk.load());
}

//-------------------------------------------------------------------------------------------------
// WorkStealing Tests

JEEVES_TEST( Heist, WorkStealing)
{
    constexpr int       kJobCount = 256;
    std::atomic< int>   completedCount{0};

    Atelier::Reset( 4);
    auto&               atelier = Atelier::Instance();
    Maestro*            mainMaestro = atelier.MainMaestro();

    for ( int i = 0; i < kJobCount; ++i) {
        uint16_t        jobId = mainMaestro->ConstructJob(
            0,
            WorkPtr::FromLambda( [&completedCount]( IWorker*) {
                std::this_thread::yield(); // Induce slight delay to encourage stealing
                completedCount.fetch_add( 1, std::memory_order_relaxed);
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

    JEEVES_ASSERT_EQ( completedCount.load(), kJobCount);

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

    std::vector< float> initialData( kArraySize);
    for ( uint32_t i = 0; i < kArraySize; ++i) {
        initialData[i] = dist( rng);
    }

    std::vector< float> expected = initialData;
    std::sort( expected.begin(), expected.end());

    for ( uint32_t szThreads : { 0u, 1u, 5u }) {
        std::vector< float> data = initialData;
        USeg            seg = USeg( static_cast< uint32_t>( data.size()));

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
        JEEVES_ASSERT_EQ( data.front(), expected.front());
        JEEVES_ASSERT_EQ( data.back(), expected.back());
        JEEVES_ASSERT_EQ( data, expected);
    }
}

//-------------------------------------------------------------------------------------------------

