#pragma once

#include <cstdint>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include "core/type_descriptor.hpp"
#include "errors/mod.hpp"

namespace pymergetic::cruspy::core {

struct RegisteredType {
  TypeDescriptor descriptor;
  std::vector<FieldDescriptor> field_storage;
};

class TypeRegistry {
 public:
  static TypeRegistry& instance();

  std::uint64_t register_type(const TypeDescriptor& descriptor,
                              const FieldDescriptor* fields);
  const RegisteredType* find(std::string_view fqn) const;
  const RegisteredType* find_by_hash(std::uint64_t schema_hash) const;
  std::uint32_t registered_count() const;

 private:
  TypeRegistry() = default;

  mutable std::mutex mutex_;
  std::unordered_map<std::string, RegisteredType> by_fqn_;
  std::unordered_map<std::uint64_t, std::string> by_hash_;
};

}  // namespace pymergetic::cruspy::core
