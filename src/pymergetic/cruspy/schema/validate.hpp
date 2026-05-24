#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>

#include "errors/mod.hpp"
#include "schema/annotations.hpp"
#include "schema/field.hpp"
#include "schema/model.hpp"

namespace pymergetic::cruspy::schema {

namespace detail {

inline void throw_validation(std::string_view field_name, std::string_view message) {
  std::string formatted = "cruspy.validation:";
  formatted.append(field_name);
  formatted.push_back(' ');
  formatted.append(message);
  throw ValidationError(std::move(formatted));
}

template <typename... Annotations>
struct ge_le_scan {
  static constexpr bool has_ge = false;
  static constexpr bool has_le = false;
  static constexpr double ge = 0.0;
  static constexpr double le = 0.0;
};

template <double V, typename... Rest>
struct ge_le_scan<ge<V>, Rest...> : ge_le_scan<Rest...> {
  static constexpr bool has_ge = true;
  static constexpr double ge = V;
  static constexpr bool has_le = ge_le_scan<Rest...>::has_le;
  static constexpr double le = ge_le_scan<Rest...>::le;
};

template <double V, typename... Rest>
struct ge_le_scan<le<V>, Rest...> : ge_le_scan<Rest...> {
  static constexpr bool has_ge = ge_le_scan<Rest...>::has_ge;
  static constexpr double ge = ge_le_scan<Rest...>::ge;
  static constexpr bool has_le = true;
  static constexpr double le = V;
};

template <typename Head, typename... Rest>
struct ge_le_scan<Head, Rest...> : ge_le_scan<Rest...> {};

template <rfl::internal::StringLiteral Name, typename T, typename Annotation>
struct annotation_applier {
  static void apply(const T&) {}
};

template <rfl::internal::StringLiteral Name, typename T, std::int64_t V>
struct annotation_applier<Name, T, min<V>> {
  static void apply(const T& value) {
    if constexpr (std::is_arithmetic_v<T>) {
      if (value < V) {
        throw_validation(Name.string_view(), "must be >= " + std::to_string(V));
      }
    }
  }
};

template <rfl::internal::StringLiteral Name, std::uint32_t V>
struct annotation_applier<Name, std::string, max_len<V>> {
  static void apply(const std::string& value) {
    if (value.size() > V) {
      throw_validation(Name.string_view(),
                       "exceeds maximum length of " + std::to_string(V));
    }
  }
};

template <rfl::internal::StringLiteral Name, typename T, double V>
struct annotation_applier<Name, T, ge<V>> {
  static void apply(const T&) {}
};

template <rfl::internal::StringLiteral Name, typename T, double V>
struct annotation_applier<Name, T, le<V>> {
  static void apply(const T&) {}
};

template <rfl::internal::StringLiteral Name, typename T, rfl::internal::StringLiteral S>
struct annotation_applier<Name, T, desc<S>> {
  static void apply(const T&) {}
};

template <rfl::internal::StringLiteral Name, typename T, typename... Annotations>
void validate_ge_le(const T& value) {
  if constexpr (std::is_same_v<T, double>) {
    using scan = ge_le_scan<Annotations...>;
    if constexpr (scan::has_ge && scan::has_le) {
      if (value < scan::ge || value > scan::le) {
        throw_validation(Name.string_view(), "must be between " + std::to_string(scan::ge) +
                                                 " and " + std::to_string(scan::le));
      }
    } else if constexpr (scan::has_ge) {
      if (value < scan::ge) {
        throw_validation(Name.string_view(), "must be >= " + std::to_string(scan::ge));
      }
    } else if constexpr (scan::has_le) {
      if (value > scan::le) {
        throw_validation(Name.string_view(), "must be <= " + std::to_string(scan::le));
      }
    }
  }
}

template <rfl::internal::StringLiteral Name, typename T, typename... Annotations>
void validate_constraints(const T& value) {
  validate_ge_le<Name, T, Annotations...>(value);
  (annotation_applier<Name, T, Annotations>::apply(value), ...);
}

template <rfl::internal::StringLiteral Name, typename Inner, typename... Annotations>
void validate_optional(const std::optional<Inner>& value) {
  if (!value.has_value()) {
    return;
  }
  validate_constraints<Name, Inner, Annotations...>(value.value());
  if constexpr (is_model_type_v<Inner>) {
    validate_model(value.value());
  }
}

}  // namespace detail

template <typename Model, rfl::internal::StringLiteral Name, typename Inner,
          typename... Annotations>
void validate_field(const field<Model, Name, std::optional<Inner>, Annotations...>& member) {
  detail::validate_optional<Name, Inner, Annotations...>(member.value());
}

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
  requires is_model_type_v<T>
void validate_field(const field<Model, Name, T, Annotations...>& member) {
  detail::validate_constraints<Name, T, Annotations...>(member.value());
  validate_model(member.value());
}

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
  requires(!is_model_type_v<T>)
void validate_field(const field<Model, Name, T, Annotations...>& member) {
  detail::validate_constraints<Name, T, Annotations...>(member.value());
}

template <typename T>
  requires is_model_type_v<T>
void validate_model(const T& model) {
  model.for_each_field([](const auto& member) { validate_field(member); });
}

}  // namespace pymergetic::cruspy::schema
