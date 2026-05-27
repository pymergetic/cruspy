#pragma once

#include <array>
#include <cstddef>
#include <string>

namespace pymergetic::cruspy::module::detail {

/// ``pymergetic::cruspy::foo`` → ``pymergetic.cruspy.foo`` (runtime).
inline std::string namespace_to_full_name(const char* ns) {
    std::string out;
    if (ns == nullptr) {
        return out;
    }
    out.reserve(std::char_traits<char>::length(ns));
    for (std::size_t i = 0; ns[i] != '\0';) {
        if (ns[i] == ':' && ns[i + 1] == ':') {
            out.push_back('.');
            i += 2;
        } else {
            out.push_back(ns[i]);
            ++i;
        }
    }
    return out;
}

/// Compile-time ``::`` → ``.`` for module paths derived from ``#NS`` tokens.
consteval std::array<char, 192> ns_to_path_array(const char* ns) {
    std::array<char, 192> out{};
    std::size_t j = 0;
    for (std::size_t i = 0; ns[i] != '\0' && j + 1 < out.size(); ++i) {
        if (ns[i] == ':' && ns[i + 1] == ':') {
            out[j++] = '.';
            ++i;
        } else {
            out[j++] = ns[i];
        }
    }
    return out;
}

}  // namespace pymergetic::cruspy::module::detail
