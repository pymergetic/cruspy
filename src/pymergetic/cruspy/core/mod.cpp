#include "core/mod.hpp"

#include <memory>
#include <string>
#include <vector>

#include "core/registry.hpp"

#ifndef CRUSPY_PKG_VERSION
#define CRUSPY_PKG_VERSION "unknown"
#endif

namespace pymergetic::cruspy::core {

std::uint32_t abi_version() { return kCruspyAbiVersion; }

const char* runtime_version() { return CRUSPY_PKG_VERSION; }

std::uint64_t register_type_from_rust(std::string_view fqn, std::uint64_t schema_hash,
                                      std::uint32_t slab_size,
                                      const RustFieldDescriptor* fields,
                                      std::uint32_t field_count) {
  std::vector<FieldDescriptor> storage;
  storage.reserve(field_count);
  for (std::uint32_t idx = 0; idx < field_count; ++idx) {
    const auto& field = fields[idx];
    storage.push_back(FieldDescriptor{
        .name = field.name,
        .kind = field.kind,
        .offset = field.offset,
        .size = field.size,
    });
  }
  TypeDescriptor descriptor{
      .fqn = fqn,
      .schema_hash = schema_hash,
      .slab_size = slab_size,
      .field_count = field_count,
      .fields = storage.data(),
  };
  return TypeRegistry::instance().register_type(descriptor, storage.data());
}

}  // namespace pymergetic::cruspy::core

extern "C" std::uint32_t cruspy_abi_version() {
  return pymergetic::cruspy::core::abi_version();
}

extern "C" const char* cruspy_runtime_version() {
  return pymergetic::cruspy::core::runtime_version();
}

extern "C" std::uint32_t cruspy_registered_type_count() {
  return pymergetic::cruspy::core::TypeRegistry::instance().registered_count();
}

extern "C" std::uint64_t cruspy_register_type_simple(const char* fqn, std::uint64_t schema_hash,
                                                       std::uint32_t slab_size) {
  pymergetic::cruspy::core::TypeDescriptor descriptor{
      .fqn = fqn,
      .schema_hash = schema_hash,
      .slab_size = slab_size,
      .field_count = 0,
      .fields = nullptr,
  };
  return pymergetic::cruspy::core::TypeRegistry::instance().register_type(descriptor,
                                                                           nullptr);
}
