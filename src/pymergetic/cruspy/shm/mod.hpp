#pragma once

#include <cstdint>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

#include "errors/mod.hpp"

namespace pymergetic::cruspy::shm {

struct ShmHandle {
  std::string segment;
  std::uint64_t offset;
  std::string type_fqn;
  std::uint64_t schema_hash;
  std::uint32_t byte_size;
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
  std::string name_;
  std::size_t capacity_;
  mutable std::mutex mutex_;
  std::vector<std::uint8_t> storage_;
};

ShmArena& open_or_create(std::string_view name, std::size_t capacity);

}  // namespace pymergetic::cruspy::shm
