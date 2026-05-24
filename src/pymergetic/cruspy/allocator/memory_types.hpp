#pragma once

#include <cstdint>
#include <cstring>
#include <string>

namespace pymergetic::cruspy::allocator {

inline constexpr std::uint32_t kCruspyMemoryAbi = 1;

inline constexpr std::uint32_t kHandleFlagTyped = 0x01;
inline constexpr std::uint32_t kHandleFlagReadOnly = 0x02;
inline constexpr std::uint32_t kHandleFlagStaleCheck = 0x04;
inline constexpr std::uint32_t kHandleFlagFileBacked = 0x08;

enum class DomainKind : std::uint8_t {
  Heap = 0,
  ProcessArena = 1,
  SharedSegment = 2,
  Pool = 3,
  FileMap = 4,
  PersistentSegment = 5,
  DeviceBuffer = 6,
};

enum class DomainVisibility : std::uint8_t {
  LocalProcess = 0,
  CrossProcess = 1,
};

enum class ResidencyTier : std::uint8_t {
  Hot = 0,
  Warm = 1,
  Cold = 2,
};

enum class TransferEngine : std::uint8_t {
  CpuCopy = 0,
  MmapRemap = 1,
  Sendfile = 2,
  IoUring = 3,
  Dma = 4,
};

struct DomainId {
  std::uint64_t high;
  std::uint64_t low;

  bool operator==(const DomainId& other) const {
    return high == other.high && low == other.low;
  }

  bool is_zero() const { return high == 0 && low == 0; }
};

static_assert(sizeof(DomainId) == 16);

// EP-0019 canonical 64-byte handle ABI.
struct MemoryHandle {
  std::uint32_t abi_version;
  std::uint32_t flags;
  DomainId domain_id;
  std::uint64_t offset;
  std::uint64_t byte_size;
  std::uint64_t schema_hash;
  std::uint64_t generation;
  char type_fqn[8];
};

static_assert(sizeof(MemoryHandle) == 64);

struct MemoryView {
  const std::uint8_t* data;
  std::uint64_t byte_size;
  std::uint64_t generation;
  bool read_only;
};

struct DomainStats {
  std::string name;
  DomainId domain_id;
  DomainKind kind;
  DomainVisibility visibility;
  ResidencyTier residency_tier;
  bool active;
  std::uint64_t bytes_total;
  std::uint64_t bytes_used;
  std::uint64_t object_count;
  std::uint64_t total_slots;
  std::uint64_t used_slots;
  float fragmentation_pct;
  float fullness_pct;
  std::string backing_path;
  std::string map_mode;
  std::uint16_t capabilities;
};

inline void set_type_fqn(MemoryHandle& handle, std::string_view fqn) {
  std::memset(handle.type_fqn, 0, sizeof(handle.type_fqn));
  const auto len = std::min(fqn.size(), sizeof(handle.type_fqn) - 1);
  std::memcpy(handle.type_fqn, fqn.data(), len);
}

inline std::string type_fqn_string(const MemoryHandle& handle) {
  return std::string(handle.type_fqn, strnlen(handle.type_fqn, sizeof(handle.type_fqn)));
}

}  // namespace pymergetic::cruspy::allocator
