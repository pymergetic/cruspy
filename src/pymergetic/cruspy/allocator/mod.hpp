#pragma once

#include <cstdint>
#include <optional>
#include <vector>

#include "allocator/memory_types.hpp"

namespace pymergetic::cruspy::allocator {

struct RegistryStats {
  std::uint32_t registered_count;
  std::uint32_t domain_count;
  std::uint64_t bytes_total;
  std::uint64_t bytes_used;
  std::uint64_t object_count;
};

RegistryStats stats();
std::vector<DomainStats> domain_stats_all();
std::optional<DomainStats> domain_stats(DomainId id);
MemoryView resolve(const MemoryHandle& handle);
MemoryHandle migrate(const MemoryHandle& handle, DomainId target);
MemoryHandle transfer(const MemoryHandle& handle, DomainId target, TransferEngine engine);

}  // namespace pymergetic::cruspy::allocator
