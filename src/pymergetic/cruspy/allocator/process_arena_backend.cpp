#include "allocator/process_arena_backend.hpp"

#include <algorithm>
#include <cstring>

#include "errors/mod.hpp"

namespace pymergetic::cruspy::allocator {

ProcessArenaBackend::ProcessArenaBackend(std::string name, std::size_t capacity)
    : name_(std::move(name)),
      domain_id_(next_domain_id()),
      capacity_(capacity),
      storage_(capacity, 0),
      bytes_used_(0),
      object_count_(0) {}

DomainStats ProcessArenaBackend::stats() const {
  std::lock_guard lock(mutex_);
  const float fullness =
      capacity_ == 0 ? 0.f : static_cast<float>(bytes_used_) / static_cast<float>(capacity_);
  return DomainStats{
      .name = name_,
      .domain_id = domain_id_,
      .kind = DomainKind::ProcessArena,
      .visibility = DomainVisibility::LocalProcess,
      .residency_tier = ResidencyTier::Hot,
      .active = true,
      .bytes_total = capacity_,
      .bytes_used = bytes_used_,
      .object_count = object_count_,
      .total_slots = 0,
      .used_slots = 0,
      .fragmentation_pct = 0.f,
      .fullness_pct = fullness * 100.f,
      .backing_path = {},
      .map_mode = "none",
      .capabilities = 0,
  };
}

MemoryHandle ProcessArenaBackend::allocate(std::string_view type_fqn,
                                           std::uint64_t schema_hash,
                                           const std::uint8_t* data,
                                           std::uint32_t byte_size) {
  if (byte_size == 0) {
    throw AllocationError("cruspy.allocation: byte_size must be > 0");
  }
  std::lock_guard lock(mutex_);
  if (byte_size > capacity_) {
    throw AllocationError("cruspy.allocation: allocation exceeds arena capacity");
  }
  const std::uint64_t offset = 0;
  auto& slot = slots_[offset];
  if (slot.generation != 0) {
    ++slot.generation;
  } else {
    slot.generation = 1;
  }
  slot.byte_size = byte_size;
  slot.schema_hash = schema_hash;
  slot.type_fqn = std::string(type_fqn);
  std::memcpy(storage_.data(), data, byte_size);
  bytes_used_ = byte_size;
  object_count_ = 1;

  MemoryHandle handle{};
  handle.abi_version = kCruspyMemoryAbi;
  handle.flags = kHandleFlagTyped | kHandleFlagStaleCheck;
  handle.domain_id = domain_id_;
  handle.offset = offset;
  handle.byte_size = byte_size;
  handle.schema_hash = schema_hash;
  handle.generation = slot.generation;
  set_type_fqn(handle, type_fqn);
  return handle;
}

MemoryView ProcessArenaBackend::resolve(const MemoryHandle& handle) const {
  std::lock_guard lock(mutex_);
  if (handle.domain_id != domain_id_) {
    throw ShmError("cruspy.shm: domain mismatch");
  }
  const auto it = slots_.find(handle.offset);
  if (it == slots_.end() || it->second.generation != handle.generation) {
    throw ShmError("cruspy.shm: stale handle");
  }
  if (handle.byte_size > storage_.size()) {
    throw ShmError("cruspy.shm: handle out of bounds");
  }
  return MemoryView{
      .data = storage_.data(),
      .byte_size = handle.byte_size,
      .generation = handle.generation,
      .read_only = (handle.flags & kHandleFlagReadOnly) != 0,
  };
}

void ProcessArenaBackend::invalidate(const MemoryHandle& handle) {
  std::lock_guard lock(mutex_);
  const auto it = slots_.find(handle.offset);
  if (it != slots_.end()) {
    ++it->second.generation;
  }
}

bool ProcessArenaBackend::generation_valid(const MemoryHandle& handle) const {
  std::lock_guard lock(mutex_);
  const auto it = slots_.find(handle.offset);
  return it != slots_.end() && it->second.generation == handle.generation;
}

}  // namespace pymergetic::cruspy::allocator
