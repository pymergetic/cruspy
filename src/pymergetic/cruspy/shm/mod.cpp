#include "shm/mod.hpp"

#include "allocator/domain_registry.hpp"

namespace pymergetic::cruspy::shm {

ShmArena::ShmArena(std::string name, std::size_t capacity)
    : name_(std::move(name)), capacity_(capacity) {}

std::size_t ShmArena::used_bytes() const {
  const auto stats =
      pymergetic::cruspy::allocator::DomainRegistry::instance().domain_stats(
          backend().domain_id());
  return stats.has_value() ? stats->bytes_used : 0;
}

allocator::ProcessArenaBackend& ShmArena::backend() const {
  return pymergetic::cruspy::allocator::DomainRegistry::instance().process_arena(name_,
                                                                                 capacity_);
}

ShmHandle ShmArena::write_bytes(std::string_view type_fqn, std::uint64_t schema_hash,
                                  const std::uint8_t* data, std::uint32_t byte_size) {
  const auto handle = backend().allocate(type_fqn, schema_hash, data, byte_size);
  return from_memory_handle(name_, type_fqn, handle);
}

std::vector<std::uint8_t> ShmArena::read_bytes(const ShmHandle& handle) const {
  if (handle.segment != name_) {
    throw ShmError("cruspy.shm: handle segment mismatch");
  }
  const auto view = backend().resolve(handle.memory);
  return std::vector<std::uint8_t>(view.data, view.data + view.byte_size);
}

ShmArena& open_or_create(std::string_view name, std::size_t capacity) {
  static std::unordered_map<std::string, std::unique_ptr<ShmArena>> arenas;
  const std::string key(name);
  const auto it = arenas.find(key);
  if (it != arenas.end()) {
    return *it->second;
  }
  auto arena = std::make_unique<ShmArena>(key, capacity);
  auto* ptr = arena.get();
  arenas.emplace(key, std::move(arena));
  pymergetic::cruspy::allocator::DomainRegistry::instance().process_arena(name, capacity);
  return *ptr;
}

ShmHandle from_memory_handle(std::string_view segment, std::string_view type_fqn,
                             const allocator::MemoryHandle& handle) {
  ShmHandle out{};
  out.segment = std::string(segment);
  out.memory = handle;
  out.type_fqn = std::string(type_fqn);
  return out;
}

}  // namespace pymergetic::cruspy::shm
