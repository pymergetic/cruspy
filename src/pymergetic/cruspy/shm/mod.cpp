#include "shm/mod.hpp"

#include <algorithm>
#include <memory>

namespace pymergetic::cruspy::shm {

namespace {

std::unordered_map<std::string, std::unique_ptr<ShmArena>>& arena_registry() {
  static std::unordered_map<std::string, std::unique_ptr<ShmArena>> arenas;
  return arenas;
}

}  // namespace

ShmArena::ShmArena(std::string name, std::size_t capacity)
    : name_(std::move(name)), capacity_(capacity), storage_(capacity, 0) {}

std::size_t ShmArena::used_bytes() const {
  std::lock_guard lock(mutex_);
  return storage_.size();
}

ShmHandle ShmArena::write_bytes(std::string_view type_fqn, std::uint64_t schema_hash,
                                const std::uint8_t* data, std::uint32_t byte_size) {
  std::lock_guard lock(mutex_);
  if (byte_size > capacity_) {
    throw AllocationError("AllocationError: SHM slot exceeds arena capacity");
  }
  if (byte_size > storage_.size()) {
    storage_.resize(byte_size);
  }
  std::copy(data, data + byte_size, storage_.begin());
  return ShmHandle{
      .segment = name_,
      .offset = 0,
      .type_fqn = std::string(type_fqn),
      .schema_hash = schema_hash,
      .byte_size = byte_size,
  };
}

std::vector<std::uint8_t> ShmArena::read_bytes(const ShmHandle& handle) const {
  if (handle.segment != name_) {
    throw ShmError("ShmError: handle segment mismatch");
  }
  std::lock_guard lock(mutex_);
  if (handle.byte_size > storage_.size()) {
    throw ShmError("ShmError: handle out of bounds");
  }
  return std::vector<std::uint8_t>(storage_.begin(),
                                   storage_.begin() + handle.byte_size);
}

ShmArena& open_or_create(std::string_view name, std::size_t capacity) {
  auto& arenas = arena_registry();
  const std::string key(name);
  const auto it = arenas.find(key);
  if (it != arenas.end()) {
    return *it->second;
  }
  auto arena = std::make_unique<ShmArena>(key, capacity);
  auto* ptr = arena.get();
  arenas.emplace(key, std::move(arena));
  return *ptr;
}

}  // namespace pymergetic::cruspy::shm
