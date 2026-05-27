#pragma once

#include "hello_gen.hpp"  // IWYU pragma: export
#include "../../substrate/__init__.hpp"

#include <cstddef>
#include <cstdint>

namespace pymergetic::cruspy::models::hello {

int hello_cpp(const substrate::MemoryHandle* handle, std::uint8_t* out, std::size_t capacity);

}  // namespace pymergetic::cruspy::models::hello
