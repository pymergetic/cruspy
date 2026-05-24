#pragma once

#include <cstdint>

namespace pymergetic::cruspy::allocator {

struct RegistryStats {
  std::uint32_t registered_count;
};

RegistryStats stats();

}  // namespace pymergetic::cruspy::allocator
