#pragma once

#include <string_view>

#include "schema/schema_base.hpp"

namespace pymergetic::cruspy::schema {

struct field_base : schema_base {
  ~field_base() override;

  schema_kind kind() const noexcept override { return schema_kind::Field; }

  virtual std::string_view field_name() const noexcept = 0;
};

template <typename T>
field_base* try_field(T& value) noexcept {
  return dynamic_cast<field_base*>(&value);
}

template <typename T>
const field_base* try_field(const T& value) noexcept {
  return dynamic_cast<const field_base*>(&value);
}

bool is_field(const field_base* ptr) noexcept;

}  // namespace pymergetic::cruspy::schema
