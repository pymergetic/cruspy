#pragma once

#include "../substrate/__init__.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

namespace pymergetic::cruspy::allocator {

struct DomainStats {
    substrate::DomainId domain_id{};
    std::string name;
    uint64_t bytes_used{};
    uint64_t bytes_capacity{};
    uint64_t allocation_count{};
};

class HeapDomain {
public:
    explicit HeapDomain(std::string name, substrate::DomainId id);

    substrate::DomainId domain_id() const { return domain_id_; }
    const std::string& name() const { return name_; }
    DomainStats stats() const;

    bool allocate(std::size_t size, substrate::MemoryHandle* out);
    bool deallocate(const substrate::MemoryHandle& handle);
    std::byte* resolve_bytes(const substrate::MemoryHandle& handle);

private:
    struct Slot {
        std::vector<std::byte> bytes;
        uint64_t generation{1};
        bool live{false};
    };

    std::string name_;
    substrate::DomainId domain_id_;
    mutable std::mutex mutex_;
    std::vector<Slot> slots_;
    std::size_t bump_end_{0};
};

class DomainRegistry {
public:
    static DomainRegistry& global();

    bool register_heap(const std::string& name);
    HeapDomain* find(const std::string& name);
    HeapDomain* find(substrate::DomainId id);
    std::vector<DomainStats> stats_all() const;

private:
    DomainRegistry() = default;

    mutable std::mutex mutex_;
    std::unordered_map<std::string, substrate::DomainId> name_to_id_;
    std::unordered_map<uint64_t, std::unique_ptr<HeapDomain>> domains_;
    uint64_t next_domain_low_{1};
};

}  // namespace pymergetic::cruspy::allocator

#ifdef __cplusplus
extern "C" {
#endif

int cruspy_allocator_register_heap(const char* name);
int cruspy_allocator_allocate(const char* domain_name, uint64_t size,
                              pymergetic::cruspy::substrate::MemoryHandle* out);
int cruspy_allocator_deallocate(const pymergetic::cruspy::substrate::MemoryHandle* handle);
int cruspy_allocator_stats_json(char* buffer, std::size_t capacity);

#ifdef __cplusplus
}
#endif
