//-------------------------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "silo/seg.h"
#include "silo/traits.h"
#include "silo/access.h"
#include "silo/arr.h"

#include <vector>
#include <array>
#include <string>
#include <numeric>

using namespace trellis::silo;

//-------------------------------------------------------------------------------------------------
// Seed Tests

TR_TEST( Silo, TruthTest)
{
    TR_ASSERT( true);
}

//-------------------------------------------------------------------------------------------------

TR_TEST( Silo, EqualityTest)
{
    int32_t             a = 42;
    int32_t             b = 40 + 2;

    TR_ASSERT_EQ( a, b);
}

//-------------------------------------------------------------------------------------------------

TR_TEST( Silo, InequalityTest)
{
    int32_t             val1 = 100;
    int32_t             val2 = 200;

    TR_ASSERT_NE( val1, val2);
}

//-------------------------------------------------------------------------------------------------
// USeg Tests

TR_TEST( Silo, USegOps)
{
    USeg                seg = USeg::New( 2, 5);

    TR_ASSERT_EQ( seg.Begin(), 2u);
    TR_ASSERT_EQ( seg.End(), 7u);
    TR_ASSERT_EQ( seg.Size(), 5u);
    TR_ASSERT( !seg.IsEmpty());

    USeg                r = seg.RSnip( 2);
    TR_ASSERT_EQ( r.Begin(), 2u);
    TR_ASSERT_EQ( r.Size(), 3u);

    USeg                l = seg.LSnip( 2);
    TR_ASSERT_EQ( l.Begin(), 4u);
    TR_ASSERT_EQ( l.Size(), 3u);

    uint32_t            sum = 0;
    seg.Traverse( [&]( uint32_t i) {
        sum += i;
    });
    TR_ASSERT_EQ( sum, 2u + 3u + 4u + 5u + 6u);

    bool                allGtOne = seg.Span( []( uint32_t i) {
        return i > 1;
    });
    TR_ASSERT( allGtOne);

    bool                allGtThree = seg.Span( []( uint32_t i) {
        return i > 3;
    });
    TR_ASSERT( !allGtThree);
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

TR_TEST( Silo, TraitsCore)
{
    Person              p{ "Alice" };
    Robot               r{ "R2D2" };

    // TRef immutable fat pointer
    TRef< GreetTrait>   refP = p;
    TRef< GreetTrait>   refR = r;

    static_assert( sizeof( refP) == 16);
    TR_ASSERT( refP.IsValid());
    TR_ASSERT( refR.IsValid());
    TR_ASSERT_EQ( refP->Greet( refP._Ptr), std::string( "Hello, Alice"));
    TR_ASSERT_EQ( refR->Greet( refR._Ptr), std::string( "Beep boop: R2D2"));

    // Downcast
    const Person*       downP = refP.As< Person>();
    const Robot*        downR = refP.As< Robot>();
    TR_ASSERT( downP != nullptr);
    TR_ASSERT( downR == nullptr);
    TR_ASSERT_EQ( downP->_Name, std::string( "Alice"));

    // MTRef mutable fat pointer
    MTRef< GreetTrait>  mutP = p;
    mutP->SetName( mutP._Ptr, "Bob");
    TR_ASSERT_EQ( p._Name, std::string( "Bob"));

    // TPtr owning polymorphic pointer
    TPtr< GreetTrait>   optP = Person{ "Charlie" };
    TR_ASSERT( optP.IsValid());
    TR_ASSERT_EQ( optP->Greet( optP.AsRef()._Ptr), std::string( "Hello, Charlie"));

    // TraitMeta TypeId & direct slot table verification (Option 2)
    uint32_t            idPerson = TraitMeta< GreetTrait>::Id< Person>();
    uint32_t            idRobot  = TraitMeta< GreetTrait>::Id< Robot>();
    TR_ASSERT( idPerson != idRobot);
    TR_ASSERT( TraitMeta< GreetTrait>::Count() >= 2u);

    const TraitMeta< GreetTrait>* metaPerson = TraitMeta< GreetTrait>::Get( idPerson);
    const TraitMeta< GreetTrait>* metaRobot  = TraitMeta< GreetTrait>::Get( idRobot);
    TR_ASSERT( metaPerson != nullptr);
    TR_ASSERT( metaRobot != nullptr);
    TR_ASSERT_EQ( metaPerson->_TypeId, idPerson);
    TR_ASSERT_EQ( metaRobot->_TypeId, idRobot);
}

//-------------------------------------------------------------------------------------------------
// IAccess Dynamic Facade & Higher-Order Operations

TR_TEST( Silo, IAccessOps)
{
    std::vector< int>   vec = { 10, 20, 30, 40, 50 };
    IAccess< int>       accessVec = vec;

    static_assert( sizeof( accessVec) == 16);
    TR_ASSERT( accessVec.IsValid());
    TR_ASSERT_EQ( accessVec.Size(), 5u);
    TR_ASSERT( !accessVec.IsEmpty());
    TR_ASSERT_EQ( accessVec.First(), 10);
    TR_ASSERT_EQ( accessVec.Last(), 50);
    TR_ASSERT_EQ( accessVec[2], 30);
    TR_ASSERT_EQ( accessVec.At( 3), 40);

    USeg                seg = accessVec.USeg();
    TR_ASSERT_EQ( seg.Begin(), 0u);
    TR_ASSERT_EQ( seg.Size(), 5u);

    int                 sumTraverse = 0;
    accessVec.Traverse( [&]( int val) {
        sumTraverse += val;
    });
    TR_ASSERT_EQ( sumTraverse, 150);

    bool                allPositive = accessVec.Span( []( int val) {
        return val > 0;
    });
    TR_ASSERT( allPositive);

    bool                sorted = accessVec.SortSanity( []( int a, int b) {
        return a < b;
    });
    TR_ASSERT( sorted);

    int                 sumIter = 0;
    for ( int x : accessVec)
        sumIter += x;
    TR_ASSERT_EQ( sumIter, 150);

    std::string         formatted = accessVec.Format();
    TR_ASSERT_EQ( formatted, std::string( "[10, 20, 30, 40, 50]"));

    // std::array binding
    std::array< float, 3> arr = { 1.5f, 2.5f, 3.5f };
    IAccess< float>     accessArr = arr;
    TR_ASSERT_EQ( accessArr.Size(), 3u);
    TR_ASSERT_EQ( accessArr.First(), 1.5f);
    TR_ASSERT_EQ( accessArr.Last(), 3.5f);
}

//-------------------------------------------------------------------------------------------------
// Arr Concrete Struct & Slice Operations

TR_TEST( Silo, ArrOps)
{
    static_assert( sizeof( Arr< int>) == 16);
    static_assert( !std::is_polymorphic_v< Arr< int>>);

    int                 buffer[5] = { 10, 20, 30, 40, 50 };
    Arr< int>           arr( buffer, 5);

    TR_ASSERT_EQ( arr.Size(), 5u);
    TR_ASSERT( !arr.IsEmpty());
    TR_ASSERT_EQ( arr.First(), 10);
    TR_ASSERT_EQ( arr.Last(), 50);
    TR_ASSERT_EQ( arr[2], 30);

    arr.SetAt( 0, 99);
    TR_ASSERT_EQ( arr.First(), 99);
    arr.SetAt( 0, 10);

    int                 swapVal = 77;
    arr.SwapAt( 1, swapVal);
    TR_ASSERT_EQ( arr[1], 77);
    TR_ASSERT_EQ( swapVal, 20);
    arr.SetAt( 1, 20);

    arr.Swap( 0, 4);
    TR_ASSERT_EQ( arr[0], 50);
    TR_ASSERT_EQ( arr[4], 10);
    arr.Swap( 0, 4);

    Arr< int>           lsnip = arr.LSnip( 2);
    TR_ASSERT_EQ( lsnip.Size(), 3u);
    TR_ASSERT_EQ( lsnip[0], 30);
    TR_ASSERT_EQ( lsnip[2], 50);

    Arr< int>           rsnip = arr.RSnip( 2);
    TR_ASSERT_EQ( rsnip.Size(), 3u);
    TR_ASSERT_EQ( rsnip[0], 10);
    TR_ASSERT_EQ( rsnip[2], 30);

    Arr< int>           subset = arr.Subset( 1, 3);
    TR_ASSERT_EQ( subset.Size(), 3u);
    TR_ASSERT_EQ( subset[0], 20);
    TR_ASSERT_EQ( subset[2], 40);

    int                 srcBuf[3] = { 100, 200, 300 };
    Arr< int>           srcArr( srcBuf, 3);
    arr.SwapFrom( 1, srcArr, 0, 3);
    TR_ASSERT_EQ( arr[1], 100);
    TR_ASSERT_EQ( arr[2], 200);
    TR_ASSERT_EQ( arr[3], 300);
    TR_ASSERT_EQ( srcArr[0], 20);
    TR_ASSERT_EQ( srcArr[1], 30);
    TR_ASSERT_EQ( srcArr[2], 40);
    arr.SwapFrom( 1, srcArr, 0, 3); // restore

    int                 idxBuf[6] = { 0 };
    Arr< int>           idxArr( idxBuf, 6);
    idxArr.DoIndexSetup();
    TR_ASSERT_EQ( idxArr[0], 0);
    TR_ASSERT_EQ( idxArr[5], 5);
    TR_ASSERT( idxArr.SortSanity( []( int a, int b) { return a < b; }));

    TR_ASSERT_EQ( arr.Format(), std::string( "[10, 20, 30, 40, 50]"));

    // String view
    std::string_view    msg = "Trellis";
    Arr< char>          charArr( msg);
    TR_ASSERT_EQ( charArr.Size(), 7u);
    TR_ASSERT_EQ( charArr.AsStringView(), std::string_view( "Trellis"));

    // Convert to IAccess
    IAccess< int>       access = arr.AsAccess();
    TR_ASSERT_EQ( access.Size(), 5u);
    TR_ASSERT_EQ( access[2], 30);
}

//-------------------------------------------------------------------------------------------------
// IArr Dynamic Trait Facade Mutating External Containers

TR_TEST( Silo, IArrFacade)
{
    std::vector< int>   vec = { 5, 4, 3, 2, 1 };
    IArr< int>          iarr = vec;

    TR_ASSERT( iarr.IsValid());
    TR_ASSERT_EQ( iarr.Size(), 5u);
    TR_ASSERT_EQ( iarr[0], 5);
    TR_ASSERT_EQ( iarr.Last(), 1);

    iarr.Swap( 0, 4);
    TR_ASSERT_EQ( vec[0], 1);
    TR_ASSERT_EQ( vec[4], 5);

    iarr.SetAt( 2, 99);
    TR_ASSERT_EQ( vec[2], 99);
}

//-------------------------------------------------------------------------------------------------
