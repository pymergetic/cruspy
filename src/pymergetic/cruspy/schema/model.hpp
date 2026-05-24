#pragma once

#include <cstdint>
#include <memory>
#include <span>
#include <string_view>
#include <type_traits>
#include <vector>

#include <rfl/internal/StringLiteral.hpp>

#include "schema/annotations.hpp"
#include "schema/bridge_make.hpp"
#include "schema/fields.hpp"
#include "schema/model_base.hpp"

namespace pymergetic::cruspy::schema {

template <typename T>
  requires is_model_type_v<T>
void validate_model(const T& model);

template <typename T>
  requires is_model_type_v<T>
std::vector<std::uint8_t> encode_model(const T& model, bool validate = true);

template <typename T>
  requires is_model_type_v<T>
T decode_model(std::span<const std::uint8_t> bytes, bool validate = true);

template <typename... Annotations>
struct model_description_from;

template <>
struct model_description_from<> {
  static constexpr std::string_view value = "";
};

template <rfl::internal::StringLiteral S, typename... Rest>
struct model_description_from<desc<S>, Rest...> {
  static constexpr std::string_view value = S.string_view();
};

template <typename Head, typename... Rest>
struct model_description_from<Head, Rest...> {
  static constexpr std::string_view value = model_description_from<Rest...>::value;
};

template <typename Self, rfl::internal::StringLiteral Name, typename... Annotations>
struct model;

/// Typed model: polymorphic `model_base` CRTP shell; members are `field<Self, ...>`.
template <typename Self, rfl::internal::StringLiteral Name, typename... Annotations>
struct model : model_base {
  using self_type = Self;
  using annotations = annotation_list<Annotations...>;

  std::string_view model_name() const noexcept override { return Name.string_view(); }

  std::string_view model_description() const noexcept override {
    return model_description_from<Annotations...>::value;
  }

  static constexpr std::string_view static_name() { return Name.string_view(); }
  static constexpr std::string_view static_description() {
    return model_description_from<Annotations...>::value;
  }

  template <typename Fn>
  void for_each_field(Fn&& fn) {
    using field_list = typename Self::schema_fields;
    field_list::for_each(static_cast<Self&>(*this), std::forward<Fn>(fn));
  }

  template <typename Fn>
  void for_each_field(Fn&& fn) const {
    using field_list = typename Self::schema_fields;
    field_list::for_each_const(static_cast<const Self&>(*this), std::forward<Fn>(fn));
  }

  void validate() const { validate_model(static_cast<const Self&>(*this)); }

  std::vector<std::uint8_t> encode(bool validate = true) const {
    return encode_model(static_cast<const Self&>(*this), validate);
  }

  static Self decode(std::span<const std::uint8_t> bytes, bool validate = true) {
    return decode_model<Self>(bytes, validate);
  }

  static std::unique_ptr<Self> make() { return make_model<Self>(); }

  template <typename... Args>
  static std::unique_ptr<Self> make(Args&&... args) {
    return make_model<Self>(std::forward<Args>(args)...);
  }
};

}  // namespace pymergetic::cruspy::schema
