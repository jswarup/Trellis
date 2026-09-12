//-------------------------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "silo/seg.h"
#include "silo/traits.h"
#include "silo/access.h"
#include "silo/arr.h"
#include "silo/buff.h"
#include "silo/stk.h"
#include "silo/stash.h"
#include "stalks/atm.h"

#include <vector>
#include <array>
#include <string>
#include <numeric>

using namespace trellis::silo;
using namespace trellis::stalks;

static_assert( sizeof( Buff< int>) == 16);
static_assert( !std::is_polymorphic_v< Buff< int>>);
static_assert( sizeof( Stk< int>) == 24);
static_assert( !std::is_polymorphic_v< Stk< int>>);
static_assert( sizeof( Stash< int>) == 24);
static_assert( !std::is_polymorphic_v< Stash< int>>);

//-------------------------------------------------------------------------------------------------
// Seed Tests

JEEVES_TEST( Silo, TruthTest)
{
    JEEVES_ASSERT( true);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Silo, EqualityTest)
{
    int32_t             a = 42;
    int32_t             b = 40 + 2;

    JEEVES_ASSERT_EQ( a, b);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Silo, InequalityTest)
{
    int32_t             val1 = 100;
    int32_t             val2 = 200;

    JEEVES_ASSERT_NE( val1, val2);
}

//-------------------------------------------------------------------------------------------------
// USeg Tests

JEEVES_TEST( Silo, USegOps)
{
    USeg                seg = USeg::New( 2, 5);

    JEEVES_ASSERT_EQ( seg.Begin(), 2u);
    JEEVES_ASSERT_EQ( seg.End(), 7u);
    JEEVES_ASSERT_EQ( seg.Size(), 5u);
    JEEVES_ASSERT( !seg.IsEmpty());

    USeg                r = seg.RSnip( 2);
    JEEVES_ASSERT_EQ( r.Begin(), 2u);
    JEEVES_ASSERT_EQ( r.Size(), 3u);

    USeg                l = seg.LSnip( 2);
    JEEVES_ASSERT_EQ( l.Begin(), 4u);
    JEEVES_ASSERT_EQ( l.Size(), 3u);

    uint32_t            sum = 0;
    seg.Traverse( [&]( uint32_t i) {
        sum += i;
    });
    JEEVES_ASSERT_EQ( sum, 2u + 3u + 4u + 5u + 6u);

    bool                allGtOne = seg.Span( []( uint32_t i) {
        return i > 1;
    });
    JEEVES_ASSERT( allGtOne);

    bool                allGtThree = seg.Span( []( uint32_t i) {
        return i > 3;
    });
    JEEVES_ASSERT( !allGtThree);

    // NewInf & IsEmpty
    USeg                empty = USeg::NewInf( 5);
    JEEVES_ASSERT( empty.IsEmpty());
    JEEVES_ASSERT_EQ( empty.Size(), 0u);

    // Mid
    JEEVES_ASSERT_EQ( seg.Mid(), 4u); // [2, 6] -> 2 + 4/2 = 4

    // IsWithin
    JEEVES_ASSERT( seg.IsWithin( 2));
    JEEVES_ASSERT( seg.IsWithin( 4));
    JEEVES_ASSERT( seg.IsWithin( 6));
    JEEVES_ASSERT( !seg.IsWithin( 1));
    JEEVES_ASSERT( !seg.IsWithin( 7));

    // TraverseRev
    std::vector< uint32_t> revItems;
    seg.TraverseRev( [&]( uint32_t i) {
        revItems.push_back( i);
    });
    JEEVES_ASSERT_EQ( revItems.size(), 5u);
    JEEVES_ASSERT_EQ( revItems[0], 6u);
    JEEVES_ASSERT_EQ( revItems[4], 2u);

    // Range-based for loop
    uint32_t            rangeSum = 0;
    for ( uint32_t val : seg)
        rangeSum += val;
    JEEVES_ASSERT_EQ( rangeSum, sum);

    // QSort using USeg
    std::vector< int>   sortBuf = { 40, 10, 50, 20, 30 };
    USeg                sortSeg = USeg::New( 0, static_cast< uint32_t>( sortBuf.size()));
    sortSeg.QSort(
        [&]( uint32_t a, uint32_t b) { return sortBuf[a] < sortBuf[b]; },
        [&]( uint32_t a, uint32_t b) { std::swap( sortBuf[a], sortBuf[b]); }
    );
    JEEVES_ASSERT_EQ( sortBuf[0], 10);
    JEEVES_ASSERT_EQ( sortBuf[1], 20);
    JEEVES_ASSERT_EQ( sortBuf[2], 30);
    JEEVES_ASSERT_EQ( sortBuf[3], 40);
    JEEVES_ASSERT_EQ( sortBuf[4], 50);

    // LowerBound, UpperBound, LocateBound
    // Array: [10, 20, 30, 30, 30, 40, 50]
    std::vector< int>   boundBuf = { 10, 20, 30, 30, 30, 40, 50 };
    USeg                boundSeg = USeg::New( 0, static_cast< uint32_t>( boundBuf.size()));

    uint32_t            lb = boundSeg.LowerBound( [&]( uint32_t idx) { return boundBuf[idx] < 30; });
    uint32_t            ub = boundSeg.UpperBound( [&]( uint32_t idx) { return boundBuf[idx] > 30; });
    JEEVES_ASSERT_EQ( lb, 2u);
    JEEVES_ASSERT_EQ( ub, 5u);

    USeg                located = boundSeg.LocateBound( [&]( uint32_t idx) { return boundBuf[idx] < 30; });
    JEEVES_ASSERT_EQ( located.First(), 2u);

    // BinarySearch
    auto                found = boundSeg.BinarySearch( [&]( uint32_t idx) {
        if ( boundBuf[idx] < 40) return -1;
        if ( boundBuf[idx] > 40) return 1;
        return 0;
    });
    JEEVES_ASSERT( found._Found);
    JEEVES_ASSERT_EQ( found._Index, 5u);

    auto                notFound = boundSeg.BinarySearch( [&]( uint32_t idx) {
        if ( boundBuf[idx] < 25) return -1;
        if ( boundBuf[idx] > 25) return 1;
        return 0;
    });
    JEEVES_ASSERT( !notFound._Found);
}

//-------------------------------------------------------------------------------------------------
// Custom Trait for TraitsCore Test

struct GreetTrait
{
    struct VTable
    {
        std::string     (*Greet)( const void* self);
        void            (*SetName)( void* self, const std::string& name);
    };

template < typename T>
    static constexpr VTable Bind( void) noexcept
    {
        return {
            .Greet = []( const void* s) -> std::string {
                return static_cast< const T*>( s)->Greet();
            },
            .SetName = []( void* s, const std::string& name) {
                static_cast< T*>( s)->SetName( name);
            }
        };
    }
};

struct Person
{
    std::string         _Name;

    std::string Greet( void) const
    {
        return "Hello, " + _Name;
    }

    void SetName( const std::string& name)
    {
        _Name = name;
    }
};

struct Robot
{
    std::string         _Model;

    std::string Greet( void) const
    {
        return "Beep boop: " + _Model;
    }

    void SetName( const std::string& name)
    {
        _Model = name;
    }
};

JEEVES_TEST( Silo, TraitsCore)
{
    Person              p{ "Alice" };
    Robot               r{ "R2D2" };

    // TRef immutable fat pointer
    TRef< GreetTrait>   refP = p;
    TRef< GreetTrait>   refR = r;

    static_assert( sizeof( refP) == 16);
    JEEVES_ASSERT( refP.IsValid());
    JEEVES_ASSERT( refR.IsValid());
    JEEVES_ASSERT_EQ( refP->Greet( refP._Ptr), std::string( "Hello, Alice"));
    JEEVES_ASSERT_EQ( refR->Greet( refR._Ptr), std::string( "Beep boop: R2D2"));

    // Downcast
    const Person*       downP = refP.As< Person>();
    const Robot*        downR = refP.As< Robot>();
    JEEVES_ASSERT( downP != nullptr);
    JEEVES_ASSERT( downR == nullptr);
    JEEVES_ASSERT_EQ( downP->_Name, std::string( "Alice"));

    // MTRef mutable fat pointer
    MTRef< GreetTrait>  mutP = p;
    mutP->SetName( mutP._Ptr, "Bob");
    JEEVES_ASSERT_EQ( p._Name, std::string( "Bob"));

    // TPtr owning polymorphic pointer
    TPtr< GreetTrait>   optP = Person{ "Charlie" };
    JEEVES_ASSERT( optP.IsValid());
    JEEVES_ASSERT_EQ( optP->Greet( optP.AsRef()._Ptr), std::string( "Hello, Charlie"));

    // TraitMeta TypeId & direct slot table verification (Option 2)
    uint32_t            idPerson = TraitMeta< GreetTrait>::Id< Person>();
    uint32_t            idRobot  = TraitMeta< GreetTrait>::Id< Robot>();
    JEEVES_ASSERT( idPerson != idRobot);
    JEEVES_ASSERT( TraitMeta< GreetTrait>::Count() >= 2u);

    const TraitMeta< GreetTrait>* metaPerson = TraitMeta< GreetTrait>::Get( idPerson);
    const TraitMeta< GreetTrait>* metaRobot  = TraitMeta< GreetTrait>::Get( idRobot);
    JEEVES_ASSERT( metaPerson != nullptr);
    JEEVES_ASSERT( metaRobot != nullptr);
    JEEVES_ASSERT_EQ( metaPerson->_TypeId, idPerson);
    JEEVES_ASSERT_EQ( metaRobot->_TypeId, idRobot);
}

//-------------------------------------------------------------------------------------------------
// IAccess Dynamic Facade & Higher-Order Operations

JEEVES_TEST( Silo, IAccessOps)
{
    std::vector< int>   vec = { 10, 20, 30, 40, 50 };
    IAccess< int>       accessVec = vec;

    static_assert( sizeof( accessVec) == 16);
    JEEVES_ASSERT( accessVec.IsValid());
    JEEVES_ASSERT_EQ( accessVec.Size(), 5u);
    JEEVES_ASSERT( !accessVec.IsEmpty());
    JEEVES_ASSERT_EQ( accessVec.First(), 10);
    JEEVES_ASSERT_EQ( accessVec.Last(), 50);
    JEEVES_ASSERT_EQ( accessVec[2], 30);
    JEEVES_ASSERT_EQ( accessVec.At( 3), 40);

    USeg                seg = accessVec.USeg();
    JEEVES_ASSERT_EQ( seg.Begin(), 0u);
    JEEVES_ASSERT_EQ( seg.Size(), 5u);

    int                 sumTraverse = 0;
    accessVec.Traverse( [&]( int val) {
        sumTraverse += val;
    });
    JEEVES_ASSERT_EQ( sumTraverse, 150);

    bool                allPositive = accessVec.Span( []( int val) {
        return val > 0;
    });
    JEEVES_ASSERT( allPositive);

    bool                sorted = accessVec.SortSanity( []( int a, int b) {
        return a < b;
    });
    JEEVES_ASSERT( sorted);

    int                 sumIter = 0;
    for ( int x : accessVec)
        sumIter += x;
    JEEVES_ASSERT_EQ( sumIter, 150);

    std::string         formatted = accessVec.Format();
    JEEVES_ASSERT_EQ( formatted, std::string( "[10, 20, 30, 40, 50]"));

    // std::array binding
    std::array< float, 3> arr = { 1.5f, 2.5f, 3.5f };
    IAccess< float>     accessArr = arr;
    JEEVES_ASSERT_EQ( accessArr.Size(), 3u);
    JEEVES_ASSERT_EQ( accessArr.First(), 1.5f);
    JEEVES_ASSERT_EQ( accessArr.Last(), 3.5f);
}

//-------------------------------------------------------------------------------------------------
// Arr Concrete Struct & Slice Operations

JEEVES_TEST( Silo, ArrOps)
{
    static_assert( sizeof( Arr< int>) == 16);
    static_assert( !std::is_polymorphic_v< Arr< int>>);

    int                 buffer[5] = { 10, 20, 30, 40, 50 };
    Arr< int>           arr( buffer, 5);

    JEEVES_ASSERT_EQ( arr.Size(), 5u);
    JEEVES_ASSERT( !arr.IsEmpty());
    JEEVES_ASSERT_EQ( arr.First(), 10);
    JEEVES_ASSERT_EQ( arr.Last(), 50);
    JEEVES_ASSERT_EQ( arr[2], 30);

    arr.SetAt( 0, 99);
    JEEVES_ASSERT_EQ( arr.First(), 99);
    arr.SetAt( 0, 10);

    int                 swapVal = 77;
    arr.SwapAt( 1, swapVal);
    JEEVES_ASSERT_EQ( arr[1], 77);
    JEEVES_ASSERT_EQ( swapVal, 20);
    arr.SetAt( 1, 20);

    arr.Swap( 0, 4);
    JEEVES_ASSERT_EQ( arr[0], 50);
    JEEVES_ASSERT_EQ( arr[4], 10);
    arr.Swap( 0, 4);

    Arr< int>           lsnip = arr.LSnip( 2);
    JEEVES_ASSERT_EQ( lsnip.Size(), 3u);
    JEEVES_ASSERT_EQ( lsnip[0], 30);
    JEEVES_ASSERT_EQ( lsnip[2], 50);

    Arr< int>           rsnip = arr.RSnip( 2);
    JEEVES_ASSERT_EQ( rsnip.Size(), 3u);
    JEEVES_ASSERT_EQ( rsnip[0], 10);
    JEEVES_ASSERT_EQ( rsnip[2], 30);

    Arr< int>           subset = arr.Subset( 1, 3);
    JEEVES_ASSERT_EQ( subset.Size(), 3u);
    JEEVES_ASSERT_EQ( subset[0], 20);
    JEEVES_ASSERT_EQ( subset[2], 40);

    int                 srcBuf[3] = { 100, 200, 300 };
    Arr< int>           srcArr( srcBuf, 3);
    arr.SwapFrom( 1, srcArr, 0, 3);
    JEEVES_ASSERT_EQ( arr[1], 100);
    JEEVES_ASSERT_EQ( arr[2], 200);
    JEEVES_ASSERT_EQ( arr[3], 300);
    JEEVES_ASSERT_EQ( srcArr[0], 20);
    JEEVES_ASSERT_EQ( srcArr[1], 30);
    JEEVES_ASSERT_EQ( srcArr[2], 40);
    arr.SwapFrom( 1, srcArr, 0, 3); // restore

    int                 idxBuf[6] = { 0 };
    Arr< int>           idxArr( idxBuf, 6);
    idxArr.DoIndexSetup();
    JEEVES_ASSERT_EQ( idxArr[0], 0);
    JEEVES_ASSERT_EQ( idxArr[5], 5);
    JEEVES_ASSERT( idxArr.SortSanity( []( int a, int b) { return a < b; }));

    JEEVES_ASSERT_EQ( arr.Format(), std::string( "[10, 20, 30, 40, 50]"));

    // String view
    std::string_view    msg = "Trellis";
    Arr< char>          charArr( msg);
    JEEVES_ASSERT_EQ( charArr.Size(), 7u);
    JEEVES_ASSERT_EQ( charArr.AsStringView(), std::string_view( "Trellis"));

    // Convert to IAccess
    IAccess< int>       access = arr.AsAccess();
    JEEVES_ASSERT_EQ( access.Size(), 5u);
    JEEVES_ASSERT_EQ( access[2], 30);
}

//-------------------------------------------------------------------------------------------------
// IArr Dynamic Trait Facade Mutating External Containers

JEEVES_TEST( Silo, IArrFacade)
{
    std::vector< int>   vec = { 5, 4, 3, 2, 1 };
    IArr< int>          iarr = vec;

    JEEVES_ASSERT( iarr.IsValid());
    JEEVES_ASSERT_EQ( iarr.Size(), 5u);
    JEEVES_ASSERT_EQ( iarr[0], 5);
    JEEVES_ASSERT_EQ( iarr.Last(), 1);

    iarr.Swap( 0, 4);
    JEEVES_ASSERT_EQ( vec[0], 1);
    JEEVES_ASSERT_EQ( vec[4], 5);

    iarr.SetAt( 2, 99);
    JEEVES_ASSERT_EQ( vec[2], 99);
}

//-------------------------------------------------------------------------------------------------
// Buff Owning Buffer Operations

JEEVES_TEST( Silo, BuffOps)
{
    Buff< int>          emptyBuff = Buff< int>::NewEmpty();
    JEEVES_ASSERT( emptyBuff.IsEmpty());
    JEEVES_ASSERT_EQ( emptyBuff.Size(), 0u);

    Buff< int>          fillBuff = Buff< int>::New( 4, 99);
    JEEVES_ASSERT_EQ( fillBuff.Size(), 4u);
    JEEVES_ASSERT_EQ( fillBuff.First(), 99);
    JEEVES_ASSERT_EQ( fillBuff.Last(), 99);

    Buff< int>          genBuff = Buff< int>::Create( 5, []( uint32_t i) {
        return static_cast< int>( i) * 10;
    });
    JEEVES_ASSERT_EQ( genBuff.Size(), 5u);
    JEEVES_ASSERT_EQ( genBuff[0], 0);
    JEEVES_ASSERT_EQ( genBuff[4], 40);

    Buff< int>          initBuff = { 1, 2, 3, 4, 5 };
    JEEVES_ASSERT_EQ( initBuff.Size(), 5u);
    JEEVES_ASSERT_EQ( initBuff[2], 3);

    initBuff.Resize( 8, []( uint32_t i) {
        return static_cast< int>( i) * 100;
    });
    JEEVES_ASSERT_EQ( initBuff.Size(), 8u);
    JEEVES_ASSERT_EQ( initBuff[5], 500);
    JEEVES_ASSERT_EQ( initBuff[7], 700);

    int                 extra[2] = { 800, 900 };
    initBuff.ExtendFromSlice( Arr< const int>( extra, 2));
    JEEVES_ASSERT_EQ( initBuff.Size(), 10u);
    JEEVES_ASSERT_EQ( initBuff.Last(), 900);

    int                 bufA[2] = { 10, 20 };
    int                 bufB[3] = { 30, 40, 50 };
    Buff< int>          concatBuff = Buff< int>::Concat( Arr< const int>( bufA, 2), Arr< const int>( bufB, 3));
    JEEVES_ASSERT_EQ( concatBuff.Size(), 5u);
    JEEVES_ASSERT_EQ( concatBuff[0], 10);
    JEEVES_ASSERT_EQ( concatBuff[4], 50);

    Arr< int>           buffArr = concatBuff;
    JEEVES_ASSERT_EQ( buffArr.Size(), 5u);

    IAccess< int>       accessBuff = concatBuff.AsAccess();
    JEEVES_ASSERT_EQ( accessBuff.Size(), 5u);
    JEEVES_ASSERT_EQ( accessBuff[1], 20);

    IArr< int>          iarrBuff = concatBuff.AsIArr();
    iarrBuff.Swap( 0, 4);
    JEEVES_ASSERT_EQ( concatBuff[0], 50);
    JEEVES_ASSERT_EQ( concatBuff[4], 10);

    auto                lsnip = concatBuff.LSnip( 2);
    JEEVES_ASSERT_EQ( lsnip.Size(), 3u);
    JEEVES_ASSERT_EQ( lsnip[0], 30);

    Buff< int>          copyBuff = concatBuff;
    JEEVES_ASSERT_EQ( copyBuff.Size(), concatBuff.Size());
    JEEVES_ASSERT_EQ( copyBuff[0], concatBuff[0]);

    Buff< int>          moveBuff = std::move( copyBuff);
    JEEVES_ASSERT_EQ( moveBuff.Size(), 5u);
    JEEVES_ASSERT( copyBuff.IsEmpty());

    JEEVES_ASSERT_EQ( concatBuff.Format(), std::string( "[50, 20, 30, 40, 10]"));
}

//-------------------------------------------------------------------------------------------------
// Stk Atomic Stack Operations

JEEVES_TEST( Silo, StkOps)
{
    Buff< int>                  buff = Buff< int>::Create( 10, []( uint32_t) { return 0; });
    Atm< uint32_t>              atm{0};
    Arr< int>                   arr = buff.AsArr();
    Stk< int>                   stack = Stk< int>::Create( &atm, arr);

    JEEVES_ASSERT_EQ( stack.Size(), 0u);
    JEEVES_ASSERT_EQ( stack.SzVoid(), 10u);
    JEEVES_ASSERT( stack.USeg().IsEmpty());

    for ( int i = 1; i <= 5; ++i)
        JEEVES_ASSERT( stack.Push( i));

    JEEVES_ASSERT_EQ( stack.Size(), 5u);
    JEEVES_ASSERT_EQ( stack.SzVoid(), 5u);
    JEEVES_ASSERT_EQ( stack.USeg().Size(), 5u);
    JEEVES_ASSERT_EQ( stack.ArrView().Size(), 5u);
    JEEVES_ASSERT_EQ( stack.ArrView()[0], 1);
    JEEVES_ASSERT_EQ( stack.ArrView()[4], 5);

    for ( int expected = 5; expected >= 1; --expected) {
        int                     out = 0;
        JEEVES_ASSERT( stack.Pop( out));
        JEEVES_ASSERT_EQ( out, expected);
    }
    JEEVES_ASSERT_EQ( stack.Size(), 0u);

    int                         emptyOut = 0;
    JEEVES_ASSERT( !stack.Pop( emptyOut));

    // Export and Import between stacks
    Buff< int>                  srcBuff = Buff< int>::Create( 10, []( uint32_t) { return 0; });
    Buff< int>                  dstBuff = Buff< int>::Create( 10, []( uint32_t) { return 0; });
    Atm< uint32_t>              srcAtm{0};
    Atm< uint32_t>              dstAtm{0};
    Stk< int>                   srcStk = Stk< int>::Create( &srcAtm, srcBuff.AsArr());
    Stk< int>                   dstStk = Stk< int>::Create( &dstAtm, dstBuff.AsArr());

    for ( int i = 10; i <= 50; i += 10)
        srcStk.Push( i);

    JEEVES_ASSERT_EQ( srcStk.Size(), 5u);
    JEEVES_ASSERT_EQ( dstStk.Size(), 0u);

    uint32_t                    exported = srcStk.Export( dstStk, 5);
    JEEVES_ASSERT_EQ( exported, 5u);
    JEEVES_ASSERT_EQ( srcStk.Size(), 0u);
    JEEVES_ASSERT_EQ( dstStk.Size(), 5u);

    uint32_t                    imported = srcStk.Import( dstStk, 3);
    JEEVES_ASSERT_EQ( imported, 3u);
    JEEVES_ASSERT_EQ( srcStk.Size(), 3u);
    JEEVES_ASSERT_EQ( dstStk.Size(), 2u);

    int                         popVal = 0;
    JEEVES_ASSERT( srcStk.Pop( popVal));
    JEEVES_ASSERT_EQ( popVal, 50);
}

//-------------------------------------------------------------------------------------------------
// Stash Dynamic Array & Stk Integration Operations

JEEVES_TEST( Silo, StashOps)
{
    Stash< int>         emptyStash;
    JEEVES_ASSERT( emptyStash.IsEmpty());
    JEEVES_ASSERT_EQ( emptyStash.Size(), 0u);

    Stash< int>         stash = { 10, 20, 30, 40, 50 };
    JEEVES_ASSERT_EQ( stash.Size(), 5u);
    JEEVES_ASSERT_EQ( stash[0], 10);
    JEEVES_ASSERT_EQ( stash[4], 50);
    JEEVES_ASSERT_EQ( stash.Front(), 10);
    JEEVES_ASSERT_EQ( stash.Back(), 50);

    stash.PushBack( 60);
    stash.EmplaceBack( 70);
    JEEVES_ASSERT_EQ( stash.Size(), 7u);
    JEEVES_ASSERT_EQ( stash.Back(), 70);

    int                 sum = 0;
    for ( int val : stash)
        sum += val;
    JEEVES_ASSERT_EQ( sum, 10 + 20 + 30 + 40 + 50 + 60 + 70);

    JEEVES_ASSERT( stash.PopBack());
    JEEVES_ASSERT_EQ( stash.Size(), 6u);
    JEEVES_ASSERT_EQ( stash.Back(), 60);

    Arr< int>           slice = stash.AsArr();
    JEEVES_ASSERT_EQ( slice.Size(), 6u);
    JEEVES_ASSERT_EQ( slice[0], 10);
    slice[0] = 99;
    JEEVES_ASSERT_EQ( stash[0], 99);
    stash[0] = 10;

    stash.Resize( 8, 42);
    JEEVES_ASSERT_EQ( stash.Size(), 8u);
    JEEVES_ASSERT_EQ( stash[6], 42);
    JEEVES_ASSERT_EQ( stash[7], 42);

    Stash< int>         copyStash( stash);
    JEEVES_ASSERT_EQ( copyStash.Size(), 8u);
    JEEVES_ASSERT_EQ( copyStash[2], 30);

    Stash< int>         moveStash( std::move( copyStash));
    JEEVES_ASSERT_EQ( moveStash.Size(), 8u);
    JEEVES_ASSERT_EQ( copyStash.Size(), 0u);

    // TrimBuff and ExtractBuff
    Stash< int>         dynStash;
    dynStash.Reserve( 128);
    for ( int i = 0; i < 10; ++i)
        dynStash.PushBack( i * 10);

    JEEVES_ASSERT_EQ( dynStash.Size(), 10u);
    JEEVES_ASSERT( dynStash.Capacity() >= 128u);

    dynStash.TrimBuff();
    JEEVES_ASSERT_EQ( dynStash.Capacity(), 10u);
    JEEVES_ASSERT_EQ( dynStash.Size(), 10u);
    JEEVES_ASSERT_EQ( dynStash[9], 90);

    Buff< int>          extractedBuff = dynStash.ExtractBuff();
    JEEVES_ASSERT_EQ( extractedBuff.Size(), 10u);
    JEEVES_ASSERT_EQ( extractedBuff[0], 0);
    JEEVES_ASSERT_EQ( extractedBuff[9], 90);
    JEEVES_ASSERT( dynStash.IsEmpty());
    JEEVES_ASSERT_EQ( dynStash.Capacity(), 0u);

    // Stk LIFO operations via Stash
    Stash< int>         stkStash = Stash< int>::New( 16, 0, 0);
    JEEVES_ASSERT_EQ( stkStash.Size(), 0u);
    stkStash.Push( 100);
    stkStash.Push( 200);
    JEEVES_ASSERT_EQ( stkStash.Size(), 2u);

    int                 popVal = 0;
    JEEVES_ASSERT( stkStash.Pop( popVal));
    JEEVES_ASSERT_EQ( popVal, 200);
    JEEVES_ASSERT( stkStash.Pop( popVal));
    JEEVES_ASSERT_EQ( popVal, 100);
    JEEVES_ASSERT_EQ( stkStash.Size(), 0u);
}

//-------------------------------------------------------------------------------------------------
