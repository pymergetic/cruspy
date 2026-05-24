#pragma once

#include <cstdint>
#include <string_view>
#include <span>

#include "allocator/memory_types.hpp"

namespace pymergetic::cruspy::schema {

enum class ValueKind : std::uint8_t {
  Int32 = 0,
  Int64 = 1,
  Float64 = 2,
  Bool = 3,
  String = 4,
  OptionalInt32 = 5,
};

struct FieldConstraints {
  bool has_min_int{false};
  std::int64_t min_int{0};
  bool has_max_len{false};
  std::uint32_t max_len{0};
  bool has_ge{false};
  double ge{0.0};
  bool has_le{false};
  double le{0.0};
};

struct FieldMeta {
  std::string_view name;
  std::string_view description;
  ValueKind value_kind;
  FieldConstraints constraints;
  bool optional{false};
  std::string_view nested_fqn;
};

struct ModelMeta {
  std::string_view name;
  std::string_view fqn;
  std::string_view description;
  allocator::DomainKind default_domain_kind;
  std::span<const FieldMeta> fields;
  std::uint64_t schema_hash;
};

}  // namespace pymergetic::cruspy::schema
