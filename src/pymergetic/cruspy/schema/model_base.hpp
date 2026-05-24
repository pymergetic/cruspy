#pragma once

#include <string_view>
#include <type_traits>

#include "schema/schema_base.hpp"

namespace pymergetic::cruspy::schema {

struct model_base : schema_base {
  ~model_base() override;

  schema_kind kind() const noexcept override { return schema_kind::Model; }

  virtual std::string_view model_name() const noexcept = 0;
  virtual std::string_view model_description() const noexcept = 0;
};

template <typename T>
model_base* try_model(T& value) noexcept {
  return dynamic_cast<model_base*>(&value);
}

template <typename T>
const model_base* try_model(const T& value) noexcept {
  return dynamic_cast<const model_base*>(&value);
}

bool is_model(const model_base* ptr) noexcept;

template <typename T>
struct is_model_type
    : std::bool_constant<std::is_base_of_v<model_base, std::remove_cvref_t<T>> &&
                         !std::is_same_v<std::remove_cvref_t<T>, model_base>> {};

template <typename T>
inline constexpr bool is_model_type_v = is_model_type<T>::value;

}  // namespace pymergetic::cruspy::schema
