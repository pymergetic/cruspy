#include "allocator/mod.hpp"

#include "core/registry.hpp"

namespace pymergetic::cruspy::allocator {

RegistryStats stats() {
  return RegistryStats{
      .registered_count =
          pymergetic::cruspy::core::TypeRegistry::instance().registered_count(),
  };
}

}  // namespace pymergetic::cruspy::allocator
