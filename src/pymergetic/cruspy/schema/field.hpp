#pragma once

#include <string_view>
#include <type_traits>

#include <rfl/Field.hpp>

#include "schema/annotations.hpp"
#include "schema/field_base.hpp"

namespace pymergetic::cruspy::schema {

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
struct field;

template <typename T>
struct is_field_type : std::false_type {};

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
struct is_field_type<field<Model, Name, T, Annotations...>> : std::true_type {};

template <typename T>
inline constexpr bool is_field_type_v = is_field_type<T>::value;

/// Typed model member: polymorphic `field_base` + rfl storage/reflection.
template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
struct field : field_base, rfl::Field<Name, T> {
  using model_type = Model;
  using value_type = T;
  using annotations = annotation_list<Annotations...>;

  field() : rfl::Field<Name, T>(T{}) {}
  using rfl::Field<Name, T>::Field;
  using rfl::Field<Name, T>::get;
  using rfl::Field<Name, T>::value;
  using rfl::Field<Name, T>::operator=;
  using rfl::Field<Name, T>::operator*;
  using rfl::Field<Name, T>::operator();

  std::string_view field_name() const noexcept override { return Name.string_view(); }

  static constexpr std::string_view static_name() { return Name.string_view(); }
};

}  // namespace pymergetic::cruspy::schema
