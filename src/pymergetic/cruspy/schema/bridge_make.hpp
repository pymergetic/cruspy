#pragma once

#include <cstddef>
#include <memory>
#include <optional>
#include <tuple>
#include <type_traits>
#include <utility>

#include "schema/fields.hpp"
#include "schema/model_base.hpp"

namespace pymergetic::cruspy::schema {

namespace detail {

template <auto MemberPtr, typename Model>
using member_value_t =
    typename std::remove_cvref_t<decltype(std::declval<Model&>().*MemberPtr)>::value_type;

template <typename T>
struct is_optional : std::false_type {};

template <typename T>
struct is_optional<std::optional<T>> : std::true_type {};

template <typename T>
inline constexpr bool is_optional_v = is_optional<std::remove_cvref_t<T>>::value;

template <typename T>
struct bridge_field_arg_slots;

template <typename Model, typename FieldList>
struct bridge_model_arg_slots;

template <typename Model, auto... MemberPtrs>
struct bridge_model_arg_slots<Model, fields<MemberPtrs...>> {
  static constexpr std::size_t value =
      (bridge_field_arg_slots<member_value_t<MemberPtrs, Model>>::value + ... + 0);
};

template <typename Model>
  requires is_model_type_v<Model>
struct bridge_field_arg_slots<Model> {
  static constexpr std::size_t value =
      bridge_model_arg_slots<Model, typename Model::schema_fields>::value;
};

template <typename T>
  requires is_optional_v<T>
struct bridge_field_arg_slots<T> {
  static constexpr std::size_t value = 2;
};

template <typename T>
  requires(!is_optional_v<T> && !is_model_type_v<T>)
struct bridge_field_arg_slots<T> {
  static constexpr std::size_t value = 1;
};

template <typename T, typename Field>
void assign_bridge_value(Field& field, T&& value) {
  field = std::forward<T>(value);
}

template <typename Nested, typename Tuple, std::size_t ArgIdx>
void assign_bridge_model_fields(Nested& nested, Tuple& args);

template <auto MemberPtr, typename Model, typename Tuple, std::size_t ArgIdx>
void assign_bridge_field(Model& model, Tuple& args) {
  auto& field = model.*MemberPtr;
  using value_type = member_value_t<MemberPtr, Model>;
  if constexpr (is_model_type_v<value_type>) {
    assign_bridge_model_fields<value_type, Tuple, ArgIdx>(field.value(), args);
  } else if constexpr (is_optional_v<value_type>) {
    const bool has_value = std::get<ArgIdx>(args);
    auto inner = std::get<ArgIdx + 1>(args);
    if (has_value) {
      field = value_type{std::move(inner)};
    } else {
      field = std::nullopt;
    }
  } else {
    assign_bridge_value(field, std::get<ArgIdx>(args));
  }
}

template <typename Model, typename FieldList, typename Tuple, std::size_t ArgIdx>
struct assign_bridge_fields_walk;

template <typename Model, typename Tuple, std::size_t ArgIdx>
struct assign_bridge_fields_walk<Model, fields<>, Tuple, ArgIdx> {
  static void run(Model&, Tuple&) {}
};

template <typename Model, auto HeadPtr, auto... RestPtrs, typename Tuple, std::size_t ArgIdx>
struct assign_bridge_fields_walk<Model, fields<HeadPtr, RestPtrs...>, Tuple, ArgIdx> {
  static void run(Model& model, Tuple& args) {
    assign_bridge_field<HeadPtr, Model, Tuple, ArgIdx>(model, args);
    constexpr std::size_t next =
        ArgIdx + bridge_field_arg_slots<member_value_t<HeadPtr, Model>>::value;
    assign_bridge_fields_walk<Model, fields<RestPtrs...>, Tuple, next>::run(model, args);
  }
};

template <typename Model, typename FieldList, typename Tuple, std::size_t ArgIdx>
void assign_bridge_fields(Model& model, Tuple& args) {
  assign_bridge_fields_walk<Model, FieldList, Tuple, ArgIdx>::run(model, args);
}

template <typename Nested, typename Tuple, std::size_t ArgIdx>
void assign_bridge_model_fields(Nested& nested, Tuple& args) {
  using field_list = typename Nested::schema_fields;
  assign_bridge_fields<Nested, field_list, Tuple, ArgIdx>(nested, args);
}

template <typename Self, typename Tuple>
void assign_bridge_args(Self& model, Tuple& args) {
  using field_list = typename Self::schema_fields;
  assign_bridge_fields<Self, field_list, Tuple, 0>(model, args);
}

}  // namespace detail

template <typename Self>
  requires is_model_type_v<Self>
std::unique_ptr<Self> make_model() {
  return std::make_unique<Self>();
}

template <typename Self, typename... Args>
  requires is_model_type_v<Self>
std::unique_ptr<Self> make_model(Args&&... args) {
  auto model = std::make_unique<Self>();
  auto arg_tuple = std::forward_as_tuple(std::forward<Args>(args)...);
  detail::assign_bridge_args<Self>(*model, arg_tuple);
  return model;
}

}  // namespace pymergetic::cruspy::schema
