// main.cpp ---------------------------------------------------------------------------------------

#include <iostream>
#include <string>
#include <cstring>
#include "cove/jeeves.h"

//-------------------------------------------------------------------------------------------------

int main( int argc, char* argv[])
{
    bool                isTestMode = false;
    const char*         testFilter = nullptr;
    int32_t             verbosity = 0;
    bool                assertsEnabled = true;

    for ( int i = 1; i < argc; ++i) {
        std::string     arg = argv[i];

        if ( arg == "-test") {
            isTestMode = true;
            if ( i + 1 < argc && argv[i + 1][0] != '-') {
                testFilter = argv[i + 1];
                ++i;
            }
        } else if ( arg.rfind( "--test=", 0) == 0) {
            isTestMode = true;
            testFilter = argv[i] + 7;
        } else if ( arg == "-v") {
            if ( i + 1 < argc && argv[i + 1][0] != '-') {
                verbosity = std::stoi( argv[i + 1]);
                ++i;
            } else {
                verbosity = 1;
            }
        } else if ( arg.rfind( "-v=", 0) == 0) {
            verbosity = std::stoi( arg.substr( 3));
        } else if ( arg == "-c") {
            assertsEnabled = false;
        }
    }

    if ( isTestMode)
        return JeevesRunner::Instance().RunAll( testFilter, verbosity, assertsEnabled);

    std::cout << "Trellis console app" << '\n';
    return 0;
}

//-------------------------------------------------------------------------------------------------
