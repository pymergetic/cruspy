cmake_minimum_required(VERSION 3.25)

# IDE-only target: generates build_clangd/compile_commands.json for clangd.
# The real extension is built by maturin/cargo (build.rs + cc-rs).
project(pymergetic_cruspy_clangd LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(FetchContent)
FetchContent_Declare(
    reflect_cpp
    GIT_REPOSITORY https://github.com/getml/reflect-cpp.git
    GIT_TAG v0.24.0
)
FetchContent_MakeAvailable(reflect_cpp)

set(CRUSPY_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/src/pymergetic/cruspy")
file(GLOB_RECURSE CRUSPY_CPP_SOURCES CONFIGURE_DEPENDS "${CRUSPY_ROOT}/*/_init.cpp")

add_library(cruspy_clangd OBJECT ${CRUSPY_CPP_SOURCES})
target_include_directories(cruspy_clangd PRIVATE
    "${CRUSPY_ROOT}"
    "${reflect_cpp_SOURCE_DIR}/include"
)
foreach(src IN LISTS CRUSPY_CPP_SOURCES)
    get_filename_component(src_dir "${src}" DIRECTORY)
    target_include_directories(cruspy_clangd PRIVATE "${src_dir}")
endforeach()

target_compile_options(cruspy_clangd PRIVATE -Wall -Wextra -Wno-unused-parameter)
