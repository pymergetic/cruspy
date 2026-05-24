#include "allocator/substrate_api.hpp"

#include <cstring>
#include <string>
#include <vector>

#include "allocator/domain_registry.hpp"
#include "errors/mod.hpp"

namespace {

thread_local std::string g_last_error;

void set_last_error(const std::exception& ex) {
  g_last_error = ex.what();
}

void copy_string(char* dest, std::size_t capacity, std::string_view value) {
  if (capacity == 0) {
    return;
  }
  std::memset(dest, 0, capacity);
  const auto len = std::min(value.size(), capacity - 1);
  std::memcpy(dest, value.data(), len);
}

const char* map_mode_string(pymergetic::cruspy::allocator::DomainKind kind,
                            std::string_view map_mode) {
  if (!map_mode.empty()) {
    return map_mode.data();
  }
  return kind == pymergetic::cruspy::allocator::DomainKind::FileMap ? "shared" : "none";
}

void fill_snapshot(const pymergetic::cruspy::allocator::DomainStats& stats,
                   CruspyDomainStatsSnapshot* out) {
  copy_string(out->name, sizeof(out->name), stats.name);
  out->domain_id_high = stats.domain_id.high;
  out->domain_id_low = stats.domain_id.low;
  out->kind = static_cast<std::uint8_t>(stats.kind);
  out->visibility = static_cast<std::uint8_t>(stats.visibility);
  out->residency_tier = static_cast<std::uint8_t>(stats.residency_tier);
  out->active = stats.active ? 1 : 0;
  out->bytes_total = stats.bytes_total;
  out->bytes_used = stats.bytes_used;
  out->object_count = stats.object_count;
  out->total_slots = stats.total_slots;
  out->used_slots = stats.used_slots;
  out->fragmentation_pct = stats.fragmentation_pct;
  out->fullness_pct = stats.fullness_pct;
  copy_string(out->backing_path, sizeof(out->backing_path), stats.backing_path);
  copy_string(out->map_mode, sizeof(out->map_mode),
              map_mode_string(stats.kind, stats.map_mode));
  out->capabilities = stats.capabilities;
}

}  // namespace

extern "C" std::uint32_t cruspy_memory_abi() {
  return pymergetic::cruspy::allocator::kCruspyMemoryAbi;
}

extern "C" std::uint32_t cruspy_domain_stats_count() {
  return static_cast<std::uint32_t>(
      pymergetic::cruspy::allocator::DomainRegistry::instance().stats().size());
}

extern "C" std::uint32_t cruspy_domain_stats_snapshot(std::uint32_t index,
                                                      CruspyDomainStatsSnapshot* out) {
  if (out == nullptr) {
    return 0;
  }
  const auto stats = pymergetic::cruspy::allocator::DomainRegistry::instance().stats();
  if (index >= stats.size()) {
    return 0;
  }
  fill_snapshot(stats[index], out);
  return 1;
}

extern "C" std::uint32_t cruspy_domain_stats_by_id(std::uint64_t domain_id_high,
                                                   std::uint64_t domain_id_low,
                                                   CruspyDomainStatsSnapshot* out) {
  if (out == nullptr) {
    return 0;
  }
  const pymergetic::cruspy::allocator::DomainId id{.high = domain_id_high, .low = domain_id_low};
  const auto stats =
      pymergetic::cruspy::allocator::DomainRegistry::instance().domain_stats(id);
  if (!stats.has_value()) {
    return 0;
  }
  fill_snapshot(*stats, out);
  return 1;
}

extern "C" std::int32_t cruspy_process_arena_open(const char* name, std::uint64_t capacity) {
  if (name == nullptr) {
    g_last_error = "cruspy.allocation: null argument";
    return -1;
  }
  try {
    pymergetic::cruspy::allocator::DomainRegistry::instance().process_arena(
        name, static_cast<std::size_t>(capacity));
    return 0;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" std::int32_t cruspy_process_arena_allocate(
    const char* name, std::uint64_t capacity, const char* type_fqn, std::uint64_t schema_hash,
    const std::uint8_t* data, std::uint32_t byte_size,
    pymergetic::cruspy::allocator::MemoryHandle* out) {
  if (name == nullptr || type_fqn == nullptr || data == nullptr || out == nullptr) {
    g_last_error = "cruspy.allocation: null argument";
    return -1;
  }
  try {
    auto& arena = pymergetic::cruspy::allocator::DomainRegistry::instance().process_arena(
        name, static_cast<std::size_t>(capacity));
    *out = arena.allocate(type_fqn, schema_hash, data, byte_size);
    return 0;
  } catch (const pymergetic::cruspy::AllocationError& ex) {
    set_last_error(ex);
    return -2;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" std::int32_t cruspy_resolve(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                                       std::uint8_t* out_data, std::uint32_t out_capacity,
                                       std::uint32_t* out_size) {
  if (handle == nullptr || out_data == nullptr || out_size == nullptr) {
    g_last_error = "cruspy.shm: null argument";
    return -1;
  }
  try {
    const auto view =
        pymergetic::cruspy::allocator::DomainRegistry::instance().resolve(*handle);
    if (view.byte_size > out_capacity) {
      g_last_error = "cruspy.shm: output buffer too small";
      return -3;
    }
    std::memcpy(out_data, view.data, view.byte_size);
    *out_size = static_cast<std::uint32_t>(view.byte_size);
    return 0;
  } catch (const pymergetic::cruspy::ShmError& ex) {
    set_last_error(ex);
    return -2;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" std::int32_t cruspy_migrate(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                                       std::uint64_t target_domain_high,
                                       std::uint64_t target_domain_low,
                                       pymergetic::cruspy::allocator::MemoryHandle* out) {
  if (handle == nullptr || out == nullptr) {
    g_last_error = "cruspy.shm: null argument";
    return -1;
  }
  try {
    const pymergetic::cruspy::allocator::DomainId target{
        .high = target_domain_high,
        .low = target_domain_low,
    };
    *out = pymergetic::cruspy::allocator::DomainRegistry::instance().migrate(*handle, target);
    return 0;
  } catch (const pymergetic::cruspy::ShmError& ex) {
    set_last_error(ex);
    return -2;
  } catch (const pymergetic::cruspy::AllocationError& ex) {
    set_last_error(ex);
    return -3;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" std::int32_t cruspy_transfer(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                                        std::uint64_t target_domain_high,
                                        std::uint64_t target_domain_low, std::uint8_t engine,
                                        pymergetic::cruspy::allocator::MemoryHandle* out) {
  if (handle == nullptr || out == nullptr) {
    g_last_error = "cruspy.shm: null argument";
    return -1;
  }
  try {
    const pymergetic::cruspy::allocator::DomainId target{
        .high = target_domain_high,
        .low = target_domain_low,
    };
    const auto transfer_engine =
        static_cast<pymergetic::cruspy::allocator::TransferEngine>(engine);
    *out = pymergetic::cruspy::allocator::DomainRegistry::instance().transfer(
        *handle, target, transfer_engine);
    return 0;
  } catch (const pymergetic::cruspy::ShmError& ex) {
    set_last_error(ex);
    return -2;
  } catch (const pymergetic::cruspy::AllocationError& ex) {
    set_last_error(ex);
    return -3;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" std::int32_t cruspy_invalidate_handle(
    const pymergetic::cruspy::allocator::MemoryHandle* handle) {
  if (handle == nullptr) {
    g_last_error = "cruspy.shm: null argument";
    return -1;
  }
  try {
    auto* backend = pymergetic::cruspy::allocator::DomainRegistry::instance().find(
        handle->domain_id);
    if (backend == nullptr) {
      g_last_error = "cruspy.shm: unknown domain";
      return -2;
    }
    backend->invalidate(*handle);
    return 0;
  } catch (const std::exception& ex) {
    set_last_error(ex);
    return -1;
  }
}

extern "C" const char* cruspy_substrate_last_error() {
  return g_last_error.c_str();
}
