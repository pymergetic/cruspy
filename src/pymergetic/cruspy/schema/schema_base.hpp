#pragma once

#include <cstdint>

namespace pymergetic::cruspy::schema {

enum class schema_kind : std::uint8_t { Field, Model };

/// Root polymorphic base for fields and models (vtable anchor in schema_base.cpp).
struct schema_base {
  virtual ~schema_base();

  virtual schema_kind kind() const noexcept = 0;
};

template <typename T>
schema_base* try_schema(T& value) noexcept {
  return dynamic_cast<schema_base*>(&value);
}

template <typename T>
const schema_base* try_schema(const T& value) noexcept {
  return dynamic_cast<const schema_base*>(&value);
}

bool is_schema(const schema_base* ptr) noexcept;

}  // namespace pymergetic::cruspy::schema
