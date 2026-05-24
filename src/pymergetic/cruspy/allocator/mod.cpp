#include "allocator/mod.hpp"

#include "allocator/domain_registry.hpp"
#include "core/registry.hpp"

namespace pymergetic::cruspy::allocator {

RegistryStats stats() {
  const auto domains = DomainRegistry::instance().stats();
  std::uint64_t bytes_total = 0;
  std::uint64_t bytes_used = 0;
  std::uint64_t object_count = 0;
  for (const auto& domain : domains) {
    bytes_total += domain.bytes_total;
    bytes_used += domain.bytes_used;
    object_count += domain.object_count;
  }
  return RegistryStats{
      .registered_count =
          pymergetic::cruspy::core::TypeRegistry::instance().registered_count(),
      .domain_count = static_cast<std::uint32_t>(domains.size()),
      .bytes_total = bytes_total,
      .bytes_used = bytes_used,
      .object_count = object_count,
  };
}

std::vector<DomainStats> domain_stats_all() {
  return DomainRegistry::instance().stats();
}

std::optional<DomainStats> domain_stats(DomainId id) {
  return DomainRegistry::instance().domain_stats(id);
}

MemoryView resolve(const MemoryHandle& handle) {
  return DomainRegistry::instance().resolve(handle);
}

MemoryHandle migrate(const MemoryHandle& handle, DomainId target) {
  return DomainRegistry::instance().migrate(handle, target);
}

MemoryHandle transfer(const MemoryHandle& handle, DomainId target, TransferEngine engine) {
  return DomainRegistry::instance().transfer(handle, target, engine);
}

}  // namespace pymergetic::cruspy::allocator
