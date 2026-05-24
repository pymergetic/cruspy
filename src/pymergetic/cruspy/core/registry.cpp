#include "core/registry.hpp"

namespace pymergetic::cruspy::core {

TypeRegistry& TypeRegistry::instance() {
  static TypeRegistry registry;
  return registry;
}

std::uint64_t TypeRegistry::register_type(const TypeDescriptor& descriptor,
                                            const FieldDescriptor* fields) {
  std::lock_guard lock(mutex_);
  const std::string fqn(descriptor.fqn);
  const auto existing = by_fqn_.find(fqn);
  if (existing != by_fqn_.end()) {
    if (existing->second.descriptor.schema_hash == descriptor.schema_hash) {
      return descriptor.schema_hash;
    }
    throw SchemaConflictError("SchemaConflictError: schema_hash mismatch for " + fqn);
  }

  RegisteredType entry;
  entry.descriptor = descriptor;
  if (descriptor.field_count > 0 && fields != nullptr) {
    entry.field_storage.assign(fields, fields + descriptor.field_count);
  }
  entry.descriptor.fields =
      entry.field_storage.empty() ? nullptr : entry.field_storage.data();
  by_fqn_.emplace(fqn, std::move(entry));
  by_hash_.emplace(descriptor.schema_hash, fqn);
  return descriptor.schema_hash;
}

const RegisteredType* TypeRegistry::find(std::string_view fqn) const {
  std::lock_guard lock(mutex_);
  const auto it = by_fqn_.find(std::string(fqn));
  if (it == by_fqn_.end()) {
    return nullptr;
  }
  return &it->second;
}

const RegisteredType* TypeRegistry::find_by_hash(std::uint64_t schema_hash) const {
  std::lock_guard lock(mutex_);
  const auto it = by_hash_.find(schema_hash);
  if (it == by_hash_.end()) {
    return nullptr;
  }
  const auto type_it = by_fqn_.find(it->second);
  if (type_it == by_fqn_.end()) {
    return nullptr;
  }
  return &type_it->second;
}

std::uint32_t TypeRegistry::registered_count() const {
  std::lock_guard lock(mutex_);
  return static_cast<std::uint32_t>(by_fqn_.size());
}

}  // namespace pymergetic::cruspy::core
