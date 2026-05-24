#pragma once

#include "../substrate/__init__.hpp"

#include <cstddef>
#include <cstdint>

namespace pymergetic::cruspy::functions {

inline constexpr uint8_t kLangCpp = 0;
inline constexpr uint8_t kLangRust = 1;
inline constexpr uint8_t kLangPython = 2;

inline constexpr uint8_t kAvailCpp = 0b001;
inline constexpr uint8_t kAvailRust = 0b010;
inline constexpr uint8_t kAvailPython = 0b100;

struct CruspyMethodSlot {
    void* cpp_fn{};
    void* rust_fn{};
    void* py_fn{};
    uint8_t available{};
    uint8_t preferred{};
    uint16_t _pad{};
};

using MethodBoolFn = bool (*)(const substrate::MemoryHandle* handle);
using MethodVoidFn = void (*)(substrate::MemoryHandle* handle);
using MethodF64Fn = double (*)(const substrate::MemoryHandle* handle, const char* arg0, const char* arg1);
using MethodBytesFn = int (*)(const substrate::MemoryHandle* handle, uint8_t* out, std::size_t capacity);
using MethodConstructorFn = int (*)(const char* fqn, substrate::MemoryHandle* out, const char* arg0, const char* arg1);
using MethodStaticStrFn = int (*)(const char* fqn, char* out, std::size_t capacity);

}  // namespace pymergetic::cruspy::functions
