cmake_minimum_required(VERSION 3.20)

project(Trellis LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/../../out)

add_executable(trellis_console
    ../../src/console/main.cpp
    ../../src/cove/jeeves.cpp
    ../../src/silo/tests/silo_tests.cpp
    ../../src/heist/tests/heist_tests.cpp
    ../../src/symph/tests/symph_tests.cpp
    ../../src/swarm/tests/swarm_tests.cpp
    ../../src/rube/tests/rube_tests.cpp
)

target_include_directories(trellis_console PRIVATE
    ../../src
)

target_compile_features(trellis_console PRIVATE cxx_std_20)
