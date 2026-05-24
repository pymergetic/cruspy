#pragma once

#include <string_view>

namespace pymergetic::cruspy {

inline constexpr std::string_view kAbiVersion = "1";

template <typename Alloc>
struct HeapAllocator {
  using value_type = Alloc;
};

template <typename Name, typename Alloc = HeapAllocator<Name>>
struct BaseModel {
  using allocator_type = Alloc;
  static constexpr std::string_view cruspy_name = "Name";
};

#define CRUSPY_MODEL(Name, ...) \
  struct Name : public ::pymergetic::cruspy::BaseModel<Name __VA_OPT__(, __VA_ARGS__)>

}  // namespace pymergetic::cruspy
