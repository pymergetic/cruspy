#include "allocator/file_map_backend.hpp"

#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#include <cstring>
#include <filesystem>

#include "errors/mod.hpp"

namespace pymergetic::cruspy::allocator {

FileMapBackend::FileMapBackend(std::string name, std::string path, std::size_t capacity)
    : name_(std::move(name)),
      path_(std::move(path)),
      domain_id_(next_domain_id()),
      capacity_(capacity),
      fd_(-1),
      mapped_(nullptr),
      bump_offset_(0),
      bytes_used_(0),
      object_count_(0) {
  std::filesystem::create_directories(std::filesystem::path(path_).parent_path());
  fd_ = ::open(path_.c_str(), O_RDWR | O_CREAT, 0644);
  if (fd_ < 0) {
    throw AllocationError("cruspy.allocation: failed to open file map backing");
  }
  if (ftruncate(fd_, static_cast<off_t>(capacity_)) != 0) {
    ::close(fd_);
    fd_ = -1;
    throw AllocationError("cruspy.allocation: failed to size file map backing");
  }
  mapped_ = static_cast<std::uint8_t*>(
      mmap(nullptr, capacity_, PROT_READ | PROT_WRITE, MAP_SHARED, fd_, 0));
  if (mapped_ == MAP_FAILED) {
    ::close(fd_);
    fd_ = -1;
    mapped_ = nullptr;
    throw AllocationError("cruspy.allocation: mmap failed");
  }
}

FileMapBackend::~FileMapBackend() {
  if (mapped_ != nullptr && mapped_ != MAP_FAILED) {
    munmap(mapped_, capacity_);
  }
  if (fd_ >= 0) {
    ::close(fd_);
  }
}

DomainStats FileMapBackend::stats() const {
  std::lock_guard lock(mutex_);
  const float fullness =
      capacity_ == 0 ? 0.f : static_cast<float>(bytes_used_) / static_cast<float>(capacity_);
  return DomainStats{
      .name = name_,
      .domain_id = domain_id_,
      .kind = DomainKind::FileMap,
      .visibility = DomainVisibility::LocalProcess,
      .residency_tier = ResidencyTier::Warm,
      .active = true,
      .bytes_total = capacity_,
      .bytes_used = bytes_used_,
      .object_count = object_count_,
      .total_slots = 0,
      .used_slots = 0,
      .fragmentation_pct = 0.f,
      .fullness_pct = fullness * 100.f,
      .backing_path = path_,
      .map_mode = "shared",
      .capabilities = 0,
  };
}

MemoryHandle FileMapBackend::allocate(std::string_view type_fqn, std::uint64_t schema_hash,
                                      const std::uint8_t* data, std::uint32_t byte_size) {
  if (byte_size == 0) {
    throw AllocationError("cruspy.allocation: byte_size must be > 0");
  }
  std::lock_guard lock(mutex_);
  if (bump_offset_ + byte_size > capacity_) {
    throw AllocationError("cruspy.allocation: file map domain full");
  }
  const std::uint64_t offset = bump_offset_;
  bump_offset_ += byte_size;
  auto& slot = slots_[offset];
  slot.generation = slot.generation == 0 ? 1 : slot.generation + 1;
  slot.byte_size = byte_size;
  slot.schema_hash = schema_hash;
  slot.type_fqn = std::string(type_fqn);
  std::memcpy(mapped_ + offset, data, byte_size);
  bytes_used_ += byte_size;
  ++object_count_;

  MemoryHandle handle{};
  handle.abi_version = kCruspyMemoryAbi;
  handle.flags = kHandleFlagTyped | kHandleFlagStaleCheck | kHandleFlagFileBacked;
  handle.domain_id = domain_id_;
  handle.offset = offset;
  handle.byte_size = byte_size;
  handle.schema_hash = schema_hash;
  handle.generation = slot.generation;
  set_type_fqn(handle, type_fqn);
  return handle;
}

MemoryView FileMapBackend::resolve(const MemoryHandle& handle) const {
  std::lock_guard lock(mutex_);
  if (handle.domain_id != domain_id_) {
    throw ShmError("cruspy.shm: domain mismatch");
  }
  const auto it = slots_.find(handle.offset);
  if (it == slots_.end() || it->second.generation != handle.generation) {
    throw ShmError("cruspy.shm: stale handle");
  }
  if (handle.offset + handle.byte_size > capacity_) {
    throw ShmError("cruspy.shm: handle out of bounds");
  }
  return MemoryView{
      .data = mapped_ + handle.offset,
      .byte_size = handle.byte_size,
      .generation = handle.generation,
      .read_only = (handle.flags & kHandleFlagReadOnly) != 0,
  };
}

void FileMapBackend::invalidate(const MemoryHandle& handle) {
  std::lock_guard lock(mutex_);
  const auto it = slots_.find(handle.offset);
  if (it != slots_.end()) {
    ++it->second.generation;
  }
}

bool FileMapBackend::generation_valid(const MemoryHandle& handle) const {
  std::lock_guard lock(mutex_);
  const auto it = slots_.find(handle.offset);
  return it != slots_.end() && it->second.generation == handle.generation;
}

}  // namespace pymergetic::cruspy::allocator
