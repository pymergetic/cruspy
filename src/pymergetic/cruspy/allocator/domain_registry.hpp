#pragma once

#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

#include "allocator/domain_backend.hpp"
#include "allocator/file_map_backend.hpp"
#include "allocator/process_arena_backend.hpp"

namespace pymergetic::cruspy::allocator {

class DomainRegistry {
 public:
  static DomainRegistry& instance();

  ProcessArenaBackend& process_arena(std::string_view name, std::size_t capacity);
  FileMapBackend& file_map(std::string_view name, std::string path, std::size_t capacity);

  DomainBackend* find(DomainId id);
  std::vector<DomainStats> stats() const;
  std::optional<DomainStats> domain_stats(DomainId id) const;

  MemoryView resolve(const MemoryHandle& handle);
  MemoryHandle migrate(const MemoryHandle& handle, DomainId target);
  MemoryHandle transfer(const MemoryHandle& handle, DomainId target, TransferEngine engine);

 private:
  DomainRegistry() = default;

  void register_backend(DomainBackend* backend);

  mutable std::mutex mutex_;
  std::unordered_map<std::string, std::unique_ptr<ProcessArenaBackend>> process_arenas_;
  std::vector<std::unique_ptr<FileMapBackend>> file_maps_;
  std::unordered_map<std::uint64_t, DomainBackend*> backends_by_low_;
};

}  // namespace pymergetic::cruspy::allocator
