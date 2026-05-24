#pragma once

#include <cstdint>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

#include "allocator/memory_types.hpp"

namespace pymergetic::cruspy::allocator {

class DomainBackend {
 public:
  virtual ~DomainBackend() = default;

  virtual DomainId domain_id() const = 0;
  virtual std::string_view name() const = 0;
  virtual DomainKind kind() const = 0;
  virtual DomainVisibility visibility() const = 0;
  virtual ResidencyTier residency_tier() const = 0;
  virtual DomainStats stats() const = 0;

  virtual MemoryHandle allocate(std::string_view type_fqn, std::uint64_t schema_hash,
                              const std::uint8_t* data, std::uint32_t byte_size) = 0;
  virtual MemoryView resolve(const MemoryHandle& handle) const = 0;
  virtual void invalidate(const MemoryHandle& handle) = 0;
  virtual bool generation_valid(const MemoryHandle& handle) const = 0;
};

DomainId next_domain_id();

}  // namespace pymergetic::cruspy::allocator
