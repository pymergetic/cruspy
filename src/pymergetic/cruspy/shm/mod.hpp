#pragma once

#include <cstdint>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include "allocator/memory_types.hpp"
#include "allocator/process_arena_backend.hpp"
#include "errors/mod.hpp"

namespace pymergetic::cruspy::shm {

struct ShmHandle {
  std::string segment;
  allocator::MemoryHandle memory;
  std::string type_fqn;
};

class ShmArena {
 public:
  explicit ShmArena(std::string name, std::size_t capacity);

  const std::string& name() const { return name_; }
  std::size_t capacity() const { return capacity_; }
  std::size_t used_bytes() const;

  ShmHandle write_bytes(std::string_view type_fqn, std::uint64_t schema_hash,
                        const std::uint8_t* data, std::uint32_t byte_size);
  std::vector<std::uint8_t> read_bytes(const ShmHandle& handle) const;

 private:
  allocator::ProcessArenaBackend& backend() const;

  std::string name_;
  std::size_t capacity_;
};

ShmArena& open_or_create(std::string_view name, std::size_t capacity);
ShmHandle from_memory_handle(std::string_view segment, std::string_view type_fqn,
                             const allocator::MemoryHandle& handle);

}  // namespace pymergetic::cruspy::shm
