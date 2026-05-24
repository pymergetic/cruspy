#pragma once

#include <cstdint>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include "allocator/domain_backend.hpp"

namespace pymergetic::cruspy::allocator {

class ProcessArenaBackend final : public DomainBackend {
 public:
  ProcessArenaBackend(std::string name, std::size_t capacity);

  DomainId domain_id() const override { return domain_id_; }
  std::string_view name() const override { return name_; }
  DomainKind kind() const override { return DomainKind::ProcessArena; }
  DomainVisibility visibility() const override { return DomainVisibility::LocalProcess; }
  ResidencyTier residency_tier() const override { return ResidencyTier::Hot; }
  DomainStats stats() const override;

  MemoryHandle allocate(std::string_view type_fqn, std::uint64_t schema_hash,
                        const std::uint8_t* data, std::uint32_t byte_size) override;
  MemoryView resolve(const MemoryHandle& handle) const override;
  void invalidate(const MemoryHandle& handle) override;
  bool generation_valid(const MemoryHandle& handle) const override;

 private:
  struct SlotMeta {
    std::uint64_t generation;
    std::uint32_t byte_size;
    std::uint64_t schema_hash;
    std::string type_fqn;
  };

  std::string name_;
  DomainId domain_id_;
  std::size_t capacity_;
  mutable std::mutex mutex_;
  std::vector<std::uint8_t> storage_;
  std::unordered_map<std::uint64_t, SlotMeta> slots_;
  std::uint64_t bytes_used_;
  std::uint64_t object_count_;
};

}  // namespace pymergetic::cruspy::allocator
