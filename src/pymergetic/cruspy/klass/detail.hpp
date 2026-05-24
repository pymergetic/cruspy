#pragma once

#include <array>
#include <cstddef>
#include <type_traits>

#include <rfl/internal/StringLiteral.hpp>
#include <rfl/internal/get_type_name.hpp>

namespace pymergetic::cruspy::klass::detail {

template <size_t N>
consteval std::size_t char_literal_last_scope(const rfl::internal::StringLiteral<N>& lit) {
    std::size_t last = static_cast<std::size_t>(-1);
    for (std::size_t i = 0; i + 1 < N; ++i) {
        if (lit.arr_[i] == ':' && lit.arr_[i + 1] == ':') {
            last = i;
            ++i;
        }
    }
    return last;
}

template <typename T>
consteval auto type_module_path_array() {
    using Type = std::remove_cvref_t<T>;
    constexpr auto type_name = rfl::internal::get_type_name<Type>();
    constexpr std::size_t last = char_literal_last_scope(type_name);
    static_assert(last != static_cast<std::size_t>(-1), "cruspy klass must live in a namespace");
    std::array<char, 192> out{};
    std::size_t j = 0;
    for (std::size_t i = 0; i < last && j + 1 < out.size(); ++i) {
        if (type_name.arr_[i] == ':' && type_name.arr_[i + 1] == ':') {
            out[j++] = '.';
            ++i;
        } else {
            out[j++] = type_name.arr_[i];
        }
    }
    return out;
}

template <typename T>
consteval auto type_short_name_array() {
    using Type = std::remove_cvref_t<T>;
    constexpr auto type_name = rfl::internal::get_type_name<Type>();
    constexpr std::size_t last = char_literal_last_scope(type_name);
    constexpr std::size_t start = last == static_cast<std::size_t>(-1) ? 0 : last + 2;
    std::array<char, 64> out{};
    std::size_t j = 0;
    for (std::size_t i = start; i + 1 < decltype(type_name)::length + 1 && type_name.arr_[i] != '\0' &&
                               j + 1 < out.size();
         ++i) {
        out[j++] = type_name.arr_[i];
    }
    return out;
}

template <typename T>
consteval auto type_fqn_array() {
    using Type = std::remove_cvref_t<T>;
    constexpr auto path = type_module_path_array<Type>();
    constexpr auto name = type_short_name_array<Type>();
    std::array<char, 256> out{};
    std::size_t j = 0;
    for (std::size_t i = 0; path[i] != '\0' && j + 1 < out.size(); ++i) {
        out[j++] = path[i];
    }
    if (j > 0 && j + 1 < out.size()) {
        out[j++] = '.';
    }
    for (std::size_t i = 0; name[i] != '\0' && j + 1 < out.size(); ++i) {
        out[j++] = name[i];
    }
    return out;
}

template <typename T>
inline const char* type_fqn() {
    static constexpr auto fqn = type_fqn_array<T>();
    return fqn.data();
}

}  // namespace pymergetic::cruspy::klass::detail

#define CRUSPY_EXPAND(x) x
#define CRUSPY_CONCAT(a, b) CRUSPY_CONCAT_I(a, b)
#define CRUSPY_CONCAT_I(a, b) a##b

#define CRUSPY_FOR_EACH_1(K, x) &K::x
#define CRUSPY_FOR_EACH_2(K, x, ...) &K::x, CRUSPY_FOR_EACH_1(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_3(K, x, ...) &K::x, CRUSPY_FOR_EACH_2(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_4(K, x, ...) &K::x, CRUSPY_FOR_EACH_3(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_5(K, x, ...) &K::x, CRUSPY_FOR_EACH_4(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_6(K, x, ...) &K::x, CRUSPY_FOR_EACH_5(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_7(K, x, ...) &K::x, CRUSPY_FOR_EACH_6(K, __VA_ARGS__)
#define CRUSPY_FOR_EACH_8(K, x, ...) &K::x, CRUSPY_FOR_EACH_7(K, __VA_ARGS__)

#define CRUSPY_NARG(...) CRUSPY_NARG_(__VA_ARGS__, 8, 7, 6, 5, 4, 3, 2, 1, 0)
#define CRUSPY_NARG_(_1, _2, _3, _4, _5, _6, _7, _8, N, ...) N

#define CRUSPY_FOR_EACH(K, ...) \
    CRUSPY_EXPAND(CRUSPY_CONCAT(CRUSPY_FOR_EACH_, CRUSPY_NARG(__VA_ARGS__))(K, __VA_ARGS__))
