#pragma once

#include <cstdint>
#include <mutex>
#include <string>
#include <unordered_map>

#include "allocator/domain_backend.hpp"

namespace pymergetic::cruspy::allocator {

class FileMapBackend final : public DomainBackend {
 public:
  FileMapBackend(std::string name, std::string path, std::size_t capacity);

  ~FileMapBackend() override;

  DomainId domain_id() const override { return domain_id_; }
  std::string_view name() const override { return name_; }
  DomainKind kind() const override { return DomainKind::FileMap; }
  DomainVisibility visibility() const override { return DomainVisibility::LocalProcess; }
  ResidencyTier residency_tier() const override { return ResidencyTier::Warm; }
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
  std::string path_;
  DomainId domain_id_;
  std::size_t capacity_;
  int fd_;
  std::uint8_t* mapped_;
  mutable std::mutex mutex_;
  std::unordered_map<std::uint64_t, SlotMeta> slots_;
  std::uint64_t bump_offset_;
  std::uint64_t bytes_used_;
  std::uint64_t object_count_;
};

}  // namespace pymergetic::cruspy::allocator
