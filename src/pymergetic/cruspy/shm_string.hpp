#pragma once

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <string>
#include <string_view>

namespace pymergetic::cruspy {

inline constexpr std::size_t kDefaultShmStringCapacity = 512;

template <std::size_t MaxLen = kDefaultShmStringCapacity>
struct ShmString {
  std::uint32_t length{0};
  char data[MaxLen]{};

  ShmString() = default;

  explicit ShmString(std::string_view value) { assign(value); }

  void assign(std::string_view value) {
    length = static_cast<std::uint32_t>(std::min(value.size(), MaxLen));
    std::memcpy(data, value.data(), length);
    if (length < MaxLen) {
      data[length] = '\0';
    }
  }

  std::string to_string() const { return std::string(data, data + length); }

  std::string_view view() const { return std::string_view(data, length); }
};

}  // namespace pymergetic::cruspy
