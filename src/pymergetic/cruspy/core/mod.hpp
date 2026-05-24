#pragma once

#include <cstdint>
#include <string>
#include <string_view>

#include "core/type_descriptor.hpp"

namespace pymergetic::cruspy::core {

inline constexpr std::uint32_t kCruspyAbiVersion = 1;

std::uint32_t abi_version();
const char* runtime_version();

struct RustFieldDescriptor {
  std::string name;
  FieldKind kind;
  std::uint32_t offset;
  std::uint32_t size;
};

std::uint64_t register_type_from_rust(std::string_view fqn, std::uint64_t schema_hash,
                                      std::uint32_t slab_size,
                                      const RustFieldDescriptor* fields,
                                      std::uint32_t field_count);

extern "C" std::uint64_t cruspy_register_type_simple(const char* fqn, std::uint64_t schema_hash,
                                                       std::uint32_t slab_size);

}  // namespace pymergetic::cruspy::core
