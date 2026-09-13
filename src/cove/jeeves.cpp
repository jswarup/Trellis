// jeeves.cpp -------------------------------------------------------------------------------------

#include "cove/jeeves.h"

#include <string>
#include <algorithm>

//-------------------------------------------------------------------------------------------------

static bool CaseInsensitiveContains( const std::string& str, const std::string& sub)
{
    if ( sub.empty())
        return true;

    auto it = std::search(
        str.begin(), str.end(),
        sub.begin(), sub.end(),
        []( char ch1, char ch2) {
            return std::tolower( static_cast< unsigned char>( ch1)) ==
                   std::tolower( static_cast< unsigned char>( ch2));
        }
    );

    return it != str.end();
}

//-------------------------------------------------------------------------------------------------

JeevesRunner& JeevesRunner::Instance()
{
    static JeevesRunner   runner = { nullptr, nullptr };
    return runner;
}

//-------------------------------------------------------------------------------------------------

void JeevesRunner::Register( JeevesCase* tc)
{
    if ( !tc)
        return;

    tc->_Next = nullptr;
    if ( !_Head) {
        _Head = tc;
        _Tail = tc;
    } else {
        _Tail->_Next = tc;
        _Tail = tc;
    }
}

//-------------------------------------------------------------------------------------------------

int JeevesRunner::RunAll( const char* filter, int32_t verbosity, bool assertsEnabled)
{
    int32_t             totalTests = 0;
    int32_t             passedTests = 0;
    int32_t             failedTests = 0;
    int32_t             skippedTests = 0;
    std::string         filterStr = ( filter ? filter : "");

    JeevesCase*           curr = _Head;

    while ( curr) {
        std::string     fullName = std::string( curr->_Suite) + "::" + curr->_Name;
        bool            matches = filterStr.empty() ||
                                  CaseInsensitiveContains( fullName, filterStr);

        if ( !matches) {
            skippedTests++;
            curr = curr->_Next;
            continue;
        }

        totalTests++;

        TestContext     ctx;
        ctx._Verbosity = verbosity;
        ctx._AssertsEnabled = assertsEnabled;
        ctx._AssertCount = 0;
        ctx._PassCount = 0;
        ctx._FailCount = 0;
        ctx._CurrentSuite = curr->_Suite;
        ctx._CurrentName = curr->_Name;

        if ( verbosity >= 2)
            std::cout << "[ RUN  ] " << fullName << '\n';

        if ( curr->_Fn)
            curr->_Fn( &ctx);

        bool isPass = ( !assertsEnabled) || ( ctx._FailCount == 0);

        if ( isPass) {
            passedTests++;
            if ( verbosity >= 1) {
                std::cout << "[ PASS ] " << fullName;
                if ( !assertsEnabled)
                    std::cout << " (assertions disabled)";
                std::cout << '\n';
            }
        } else {
            failedTests++;
            if ( verbosity >= 1)
                std::cout << "[ FAIL ] " << fullName << '\n';
        }

        curr = curr->_Next;
    }

    std::cout << totalTests << " tests run, "
              << passedTests << " passed, "
              << failedTests << " failed.";

    if ( skippedTests > 0)
        std::cout << " (" << skippedTests << " skipped)";

    if ( !assertsEnabled)
        std::cout << " (assertions disabled)";

    std::cout << '\n';

    return ( failedTests == 0) ? 0 : 1;
}

//-------------------------------------------------------------------------------------------------

