// jeeves.h -------------------------------------------------------------------------------------------------------
#pragma once

#include <cstdint>
#include <iostream>
#include <cstring>

//-------------------------------------------------------------------------------------------------
// TestContext — per-run state passed into every test function.

struct TestContext
{
    int32_t             _Verbosity;
    bool                _AssertsEnabled;
    int32_t             _AssertCount;
    int32_t             _PassCount;
    int32_t             _FailCount;
    const char*         _CurrentSuite;
    const char*         _CurrentName;
};

//-------------------------------------------------------------------------------------------------
// JeevesCase — intrusive linked-list node, one per JEEVES_TEST() invocation.

struct JeevesCase
{
    const char*         _Suite;
    const char*         _Name;
    void                (*_Fn)( TestContext*);
    JeevesCase*           _Next;
};

//-------------------------------------------------------------------------------------------------
// JeevesRunner — singleton registry and executor.

struct JeevesRunner
{
    JeevesCase*           _Head;
    JeevesCase*           _Tail;

    static JeevesRunner&  Instance();
    void                Register( JeevesCase* tc);
    int                 RunAll( const char* filter, int32_t verbosity, bool assertsEnabled);
};

//-------------------------------------------------------------------------------------------------
// JeevesRegistrar — RAII helper; its constructor auto-registers a JeevesCase
// at static-initialization time (before main).

struct JeevesRegistrar
{
    JeevesRegistrar( JeevesCase* tc)
    {
        JeevesRunner::Instance().Register( tc);
    }
};

//-------------------------------------------------------------------------------------------------
// JEEVES_TEST — declares and auto-registers a test.
//
// Usage:
//   JEEVES_TEST( SuiteName, TestName)
//   {
//       JEEVES_ASSERT( condition);
//   }

#define JEEVES_TEST( suite, name)                                       \
    static void suite##_##name##_Fn( TestContext* ctx);                 \
    static JeevesCase suite##_##name##_Case = {                           \
        #suite, #name, suite##_##name##_Fn, nullptr                    \
    };                                                                  \
    static JeevesRegistrar suite##_##name##_Reg(                          \
        &suite##_##name##_Case);                                        \
    static void suite##_##name##_Fn( TestContext* ctx)

//-------------------------------------------------------------------------------------------------
// JEEVES_ASSERT — boolean assertion.
// When assertions are disabled (-c), the body still runs but the check
// is skipped. A failure does NOT abort; remaining assertions keep running.

#define JEEVES_ASSERT( cond)                                            \
    do {                                                                \
        ctx->_AssertCount++;                                            \
        if ( ctx->_AssertsEnabled) {                                    \
            if ( !(cond)) {                                             \
                ctx->_FailCount++;                                      \
                if ( ctx->_Verbosity >= 1)                              \
                    std::cerr << "         ASSERT( " << #cond           \
                              << " ) FAILED ("                          \
                              << __FILE__ << ":" << __LINE__            \
                              << ")\n";                                 \
            } else {                                                    \
                ctx->_PassCount++;                                      \
                if ( ctx->_Verbosity >= 2)                              \
                    std::cout << "         ASSERT( " << #cond           \
                              << " ) ... ok\n";                         \
            }                                                           \
        }                                                               \
    } while ( 0)

//-------------------------------------------------------------------------------------------------
// JEEVES_ASSERT_EQ — equality assertion.

#define JEEVES_ASSERT_EQ( a, b)                                         \
    do {                                                                \
        ctx->_AssertCount++;                                            \
        if ( ctx->_AssertsEnabled) {                                    \
            if ( !((a) == (b))) {                                       \
                ctx->_FailCount++;                                      \
                if ( ctx->_Verbosity >= 1)                              \
                    std::cerr << "         ASSERT_EQ( " << #a           \
                              << ", " << #b << " ) FAILED ("            \
                              << __FILE__ << ":" << __LINE__            \
                              << ")\n";                                 \
            } else {                                                    \
                ctx->_PassCount++;                                      \
                if ( ctx->_Verbosity >= 2)                              \
                    std::cout << "         ASSERT_EQ( " << #a           \
                              << ", " << #b << " ) ... ok\n";           \
            }                                                           \
        }                                                               \
    } while ( 0)

//-------------------------------------------------------------------------------------------------
// JEEVES_ASSERT_NE — inequality assertion.

#define JEEVES_ASSERT_NE( a, b)                                         \
    do {                                                                \
        ctx->_AssertCount++;                                            \
        if ( ctx->_AssertsEnabled) {                                    \
            if ( !((a) != (b))) {                                       \
                ctx->_FailCount++;                                      \
                if ( ctx->_Verbosity >= 1)                              \
                    std::cerr << "         ASSERT_NE( " << #a           \
                              << ", " << #b << " ) FAILED ("            \
                              << __FILE__ << ":" << __LINE__            \
                              << ")\n";                                 \
            } else {                                                    \
                ctx->_PassCount++;                                      \
                if ( ctx->_Verbosity >= 2)                              \
                    std::cout << "         ASSERT_NE( " << #a           \
                              << ", " << #b << " ) ... ok\n";           \
            }                                                           \
        }                                                               \
    } while ( 0)
