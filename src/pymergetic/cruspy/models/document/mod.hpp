#pragma once

// EP-0014 — sample model (phase 1 reference). Codegen target for easybind_rust_module().

#include <cstdint>
#include <string_view>

namespace pymergetic::cruspy {

template <typename T>
struct HeapAllocator {};

template <typename Name, typename Alloc = HeapAllocator<Name>>
struct BaseModel {};

#define CRUSPY_MODEL(Name, ...) struct Name

}  // namespace pymergetic::cruspy

CRUSPY_MODEL(Document) {
  int32_t id;
  float score;
  bool active;
};

#undef CRUSPY_MODEL
