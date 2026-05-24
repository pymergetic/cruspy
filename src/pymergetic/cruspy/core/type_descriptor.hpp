#pragma once

#include <cstdint>
#include <string_view>

namespace pymergetic::cruspy::core {

enum class FieldKind : std::uint8_t {
  Int32 = 0,
  Int64 = 1,
  Float64 = 2,
  Bool = 3,
  String = 4,
};

struct FieldDescriptor {
  std::string_view name;
  FieldKind kind;
  std::uint32_t offset;
  std::uint32_t size;
};

struct TypeDescriptor {
  std::string_view fqn;
  std::uint64_t schema_hash;
  std::uint32_t slab_size;
  std::uint32_t field_count;
  const FieldDescriptor* fields;
};

}  // namespace pymergetic::cruspy::core
