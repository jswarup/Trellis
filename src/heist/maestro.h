#ifndef TRELLIS_HEIST_MAESTRO_H
#define TRELLIS_HEIST_MAESTRO_H

//-------------------------------------------------------------------------------------------------

#include "silo/buff.h"
#include "silo/stash.h"
#include "stalks/atm.h"
#include "stalks/work.h"

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::heist {

class Atelier;

//-------------------------------------------------------------------------------------------------
// Maestro — per-thread worker context managing job caching, scheduling queues, and work execution.

class alignas( 64) Maestro : public stalks::IWorker
{
public:
    uint32_t                    _SzProcessed{0};

private:
    uint32_t                    _Index{0};
    Atelier*                    _Atelier{nullptr};
    silo::Stash< uint16_t>      _JobCache{};
    silo::Stash< uint16_t>      _RunQueue{};
    stalks::Spinlock            _RunQLock{};
    stalks::Atm< uint16_t>      _CurSuccId{0};
    silo::Stash< uint16_t>      _TempQueue{};

public:
    constexpr Maestro( void) noexcept = default;

    explicit Maestro( uint32_t maestroInd)
        : _Index( maestroInd),
          _JobCache( 256, 0, static_cast< uint16_t>( 0)),
          _RunQueue( 1024, 0, static_cast< uint16_t>( 0)),
          _CurSuccId( 0),
          _TempQueue( 64, 0, static_cast< uint16_t>( 0))
    {
    }

    void SetAtelier( Atelier* atelier) noexcept
    {
        _Atelier = atelier;
    }

    Atelier* AtelierRef( void) noexcept
    {
        return _Atelier;
    }

    const Atelier* AtelierRef( void) const noexcept
    {
        return _Atelier;
    }

    uint32_t MaestroIndex( void) const noexcept
    {
        return _Index;
    }

    static Maestro* FromWorker( stalks::IWorker* worker) noexcept
    {
        return static_cast< Maestro*>( worker);
    }

    uint16_t ConstructJob( uint16_t succId, stalks::WorkPtr job);

    void EnqueueJob( uint16_t jobId)
    {
        _TempQueue.Push( jobId);
    }

    uint16_t ConstructEnqueArr( uint16_t succId, silo::Buff< uint16_t> buff);

    silo::Stk< uint16_t> JobCacheStk( void) const noexcept
    {
        return _JobCache.StkView();
    }

    silo::Arr< uint16_t> RunQueueArr( void) const noexcept
    {
        return _RunQueue.StkView().ArrView();
    }

    void FlushTempQueue( void);

    void EnqueRunJob( uint16_t jobId)
    {
        auto guard = _RunQLock.Lock();
        _RunQueue.Push( jobId);
    }

    uint16_t PopJob( void)
    {
        uint16_t jobId = 0;
        if ( _RunQueue.Size() != 0) {
            auto guard = _RunQLock.Lock();
            if ( _RunQueue.Pop( jobId)) {
                return jobId;
            }
        }
        return 0;
    }

    uint16_t CurSuccId( void) const noexcept
    {
        return _CurSuccId.Load( std::memory_order_acquire);
    }

    void SetCurSuccId( uint16_t val) noexcept
    {
        _CurSuccId.Store( val, std::memory_order_release);
    }

template < typename TChoreNode>
    void PostChoreTree( const TChoreNode& node);

    void PostJob( stalks::WorkPtr job) override;
};

} // namespace trellis::heist

//-------------------------------------------------------------------------------------------------

#endif // TRELLIS_HEIST_MAESTRO_H

