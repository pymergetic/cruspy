#pragma once

#include <cstdint>

#include "allocator/memory_types.hpp"

extern "C" {

inline constexpr std::uint32_t CRUSPY_MEMORY_ABI = pymergetic::cruspy::allocator::kCruspyMemoryAbi;

struct CruspyDomainStatsSnapshot {
  char name[64];
  std::uint64_t domain_id_high;
  std::uint64_t domain_id_low;
  std::uint8_t kind;
  std::uint8_t visibility;
  std::uint8_t residency_tier;
  std::uint8_t active;
  std::uint64_t bytes_total;
  std::uint64_t bytes_used;
  std::uint64_t object_count;
  std::uint64_t total_slots;
  std::uint64_t used_slots;
  float fragmentation_pct;
  float fullness_pct;
  char backing_path[256];
  char map_mode[16];
  std::uint16_t capabilities;
};

std::uint32_t cruspy_memory_abi();

std::uint32_t cruspy_domain_stats_count();
std::uint32_t cruspy_domain_stats_snapshot(std::uint32_t index,
                                           CruspyDomainStatsSnapshot* out);
std::uint32_t cruspy_domain_stats_by_id(std::uint64_t domain_id_high,
                                        std::uint64_t domain_id_low,
                                        CruspyDomainStatsSnapshot* out);

std::int32_t cruspy_process_arena_open(const char* name, std::uint64_t capacity);

std::int32_t cruspy_process_arena_allocate(const char* name, std::uint64_t capacity,
                                           const char* type_fqn, std::uint64_t schema_hash,
                                           const std::uint8_t* data, std::uint32_t byte_size,
                                           pymergetic::cruspy::allocator::MemoryHandle* out);

std::int32_t cruspy_resolve(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                            std::uint8_t* out_data, std::uint32_t out_capacity,
                            std::uint32_t* out_size);

std::int32_t cruspy_migrate(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                            std::uint64_t target_domain_high, std::uint64_t target_domain_low,
                            pymergetic::cruspy::allocator::MemoryHandle* out);

std::int32_t cruspy_transfer(const pymergetic::cruspy::allocator::MemoryHandle* handle,
                             std::uint64_t target_domain_high, std::uint64_t target_domain_low,
                             std::uint8_t engine,
                             pymergetic::cruspy::allocator::MemoryHandle* out);

std::int32_t cruspy_invalidate_handle(
    const pymergetic::cruspy::allocator::MemoryHandle* handle);

const char* cruspy_substrate_last_error();

}  // extern "C"
