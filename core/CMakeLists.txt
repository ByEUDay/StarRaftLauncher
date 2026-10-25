cmake_minimum_required(VERSION 3.16)
project(starraft_core VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

option(SR_CORE_BUILD_SHARED "Build starraft_core as a shared library" OFF)
option(SR_CORE_BUILD_TESTS "Build core unit tests" OFF)

set(SR_CORE_SOURCES
    src/core.cpp
    src/sha1.cpp
    src/sysinfo.cpp
    src/java_detect.cpp
)

if(SR_CORE_BUILD_SHARED)
    add_library(starraft_core SHARED ${SR_CORE_SOURCES})
    target_compile_definitions(starraft_core PRIVATE SR_CORE_SHARED SR_CORE_EXPORTS)
else()
    add_library(starraft_core STATIC ${SR_CORE_SOURCES})
endif()

target_include_directories(starraft_core PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

if(MSVC)
    target_compile_options(starraft_core PRIVATE /W4 /utf-8)
else()
    target_compile_options(starraft_core PRIVATE -Wall -Wextra)
endif()

install(TARGETS starraft_core ARCHIVE DESTINATION lib LIBRARY DESTINATION lib RUNTIME DESTINATION bin)
install(FILES include/starraft/core.h DESTINATION include/starraft)

if(SR_CORE_BUILD_TESTS)
    enable_testing()
    add_executable(starraft_core_tests tests/test_core.cpp)
    target_link_libraries(starraft_core_tests PRIVATE starraft_core)
    add_test(NAME starraft_core_tests COMMAND starraft_core_tests)
endif()
