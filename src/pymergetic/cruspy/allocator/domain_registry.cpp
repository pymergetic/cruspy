#include "allocator/domain_registry.hpp"

#include "errors/mod.hpp"

namespace pymergetic::cruspy::allocator {

namespace {

DomainBackend* require_backend(DomainRegistry& registry, DomainId id) {
  auto* backend = registry.find(id);
  if (backend == nullptr) {
    throw ShmError("cruspy.shm: unknown domain");
  }
  return backend;
}

}  // namespace

DomainRegistry& DomainRegistry::instance() {
  static DomainRegistry registry;
  return registry;
}

void DomainRegistry::register_backend(DomainBackend* backend) {
  backends_by_low_[backend->domain_id().low] = backend;
}

ProcessArenaBackend& DomainRegistry::process_arena(std::string_view name, std::size_t capacity) {
  std::lock_guard lock(mutex_);
  const std::string key(name);
  const auto it = process_arenas_.find(key);
  if (it != process_arenas_.end()) {
    return *it->second;
  }
  auto arena = std::make_unique<ProcessArenaBackend>(key, capacity);
  auto* ptr = arena.get();
  register_backend(ptr);
  process_arenas_.emplace(key, std::move(arena));
  return *ptr;
}

FileMapBackend& DomainRegistry::file_map(std::string_view name, std::string path,
                                         std::size_t capacity) {
  std::lock_guard lock(mutex_);
  auto backend = std::make_unique<FileMapBackend>(std::string(name), std::move(path), capacity);
  auto* ptr = backend.get();
  register_backend(ptr);
  file_maps_.push_back(std::move(backend));
  return *ptr;
}

DomainBackend* DomainRegistry::find(DomainId id) {
  std::lock_guard lock(mutex_);
  const auto it = backends_by_low_.find(id.low);
  if (it == backends_by_low_.end()) {
    return nullptr;
  }
  if (it->second->domain_id() != id) {
    return nullptr;
  }
  return it->second;
}

std::vector<DomainStats> DomainRegistry::stats() const {
  std::lock_guard lock(mutex_);
  std::vector<DomainStats> out;
  out.reserve(process_arenas_.size() + file_maps_.size());
  for (const auto& [_, arena] : process_arenas_) {
    out.push_back(arena->stats());
  }
  for (const auto& backend : file_maps_) {
    out.push_back(backend->stats());
  }
  return out;
}

std::optional<DomainStats> DomainRegistry::domain_stats(DomainId id) const {
  std::lock_guard lock(mutex_);
  const auto it = backends_by_low_.find(id.low);
  if (it == backends_by_low_.end() || it->second->domain_id() != id) {
    return std::nullopt;
  }
  return it->second->stats();
}

MemoryView DomainRegistry::resolve(const MemoryHandle& handle) {
  auto* backend = require_backend(*this, handle.domain_id);
  return backend->resolve(handle);
}

MemoryHandle DomainRegistry::migrate(const MemoryHandle& handle, DomainId target) {
  return transfer(handle, target, TransferEngine::CpuCopy);
}

MemoryHandle DomainRegistry::transfer(const MemoryHandle& handle, DomainId target,
                                      TransferEngine engine) {
  auto* source = require_backend(*this, handle.domain_id);
  if (!source->generation_valid(handle)) {
    throw ShmError("cruspy.shm: stale handle");
  }
  auto* destination = require_backend(*this, target);
  if (destination == source) {
    return handle;
  }

  const MemoryView view = source->resolve(handle);
  if (engine != TransferEngine::CpuCopy) {
    // Accelerated engines are optional; CpuCopy is always correct.
    engine = TransferEngine::CpuCopy;
  }

  const std::string type_fqn = type_fqn_string(handle);
  MemoryHandle migrated = destination->allocate(type_fqn, handle.schema_hash, view.data,
                                                static_cast<std::uint32_t>(view.byte_size));
  source->invalidate(handle);
  return migrated;
}

}  // namespace pymergetic::cruspy::allocator
