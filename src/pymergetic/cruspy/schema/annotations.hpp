#pragma once

#include <cstdint>
#include <string_view>

#include <rfl/internal/StringLiteral.hpp>

namespace pymergetic::cruspy::schema {

template <std::int64_t V>
struct min {
  static constexpr std::int64_t value = V;
};

template <std::uint32_t V>
struct max_len {
  static constexpr std::uint32_t value = V;
};

template <double V>
struct ge {
  static constexpr double value = V;
};

template <double V>
struct le {
  static constexpr double value = V;
};

template <rfl::internal::StringLiteral S>
struct desc {
  static constexpr std::string_view value = S.string_view();
};

template <typename... Ts>
struct annotation_list {};

}  // namespace pymergetic::cruspy::schema
