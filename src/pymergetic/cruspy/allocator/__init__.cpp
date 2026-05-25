#include "__init__.hpp"

#include "../module/__init__.hpp"

#include <cstdio>
#include <cstring>

namespace pymergetic::cruspy::allocator {
namespace {

substrate::DomainId make_id(uint64_t low) {
    substrate::DomainId id{};
    id.high = 0x637275737079;  // "cruspy"
    id.low = low;
    return id;
}

}  // namespace

HeapDomain::HeapDomain(std::string name, substrate::DomainId id)
    : name_(std::move(name)), domain_id_(id) {}

DomainStats HeapDomain::stats() const {
    std::lock_guard lock(mutex_);
    DomainStats s{};
    s.domain_id = domain_id_;
    s.name = name_;
    s.bytes_capacity = 0;
    s.bytes_used = bump_end_;
    for (const auto& slot : slots_) {
        if (slot.live) {
            s.allocation_count += 1;
        }
    }
    return s;
}

bool HeapDomain::allocate(std::size_t size, substrate::MemoryHandle* out) {
    if (out == nullptr || size == 0) {
        return false;
    }
    std::lock_guard lock(mutex_);
    const std::size_t index = slots_.size();
    Slot slot;
    slot.bytes.resize(size);
    slot.live = true;
    slots_.push_back(std::move(slot));
    bump_end_ += size;

    substrate::handle_zero(out);
    out->domain_id = domain_id_;
    out->offset = index;
    out->byte_size = size;
    out->generation = slots_.back().generation;
    return true;
}

bool HeapDomain::deallocate(const substrate::MemoryHandle& handle) {
    std::lock_guard lock(mutex_);
    if (handle.domain_id != domain_id_) {
        return false;
    }
    if (handle.offset >= slots_.size()) {
        return false;
    }
    auto& slot = slots_[handle.offset];
    if (!slot.live || slot.generation != handle.generation) {
        return false;
    }
    slot.live = false;
    slot.generation += 1;
    return true;
}

std::byte* HeapDomain::resolve_bytes(const substrate::MemoryHandle& handle) {
    std::lock_guard lock(mutex_);
    if (handle.domain_id != domain_id_ || handle.offset >= slots_.size()) {
        return nullptr;
    }
    auto& slot = slots_[handle.offset];
    if (!slot.live || slot.generation != handle.generation) {
        return nullptr;
    }
    return slot.bytes.data();
}

DomainRegistry& DomainRegistry::global() {
    static DomainRegistry registry;
    return registry;
}

bool DomainRegistry::register_heap(const std::string& name) {
    std::lock_guard lock(mutex_);
    if (name_to_id_.contains(name)) {
        return true;
    }
    const auto id = make_id(next_domain_low_++);
    domains_.emplace(id.low, std::make_unique<HeapDomain>(name, id));
    name_to_id_.emplace(name, id);
    return true;
}

bool DomainRegistry::register_shm(const std::string& name, ShmDomainOps ops, uint64_t* domain_low_out) {
    std::lock_guard lock(mutex_);
    if (name_to_id_.contains(name)) {
        if (domain_low_out != nullptr) {
            *domain_low_out = name_to_id_[name].low;
        }
        return true;
    }
    const auto id = make_id(next_domain_low_++);
    shm_domains_.emplace(name, ops);
    shm_id_to_name_.emplace(id.low, name);
    name_to_id_.emplace(name, id);
    if (domain_low_out != nullptr) {
        *domain_low_out = id.low;
    }
    return true;
}

bool DomainRegistry::allocate(const std::string& name, std::size_t size, substrate::MemoryHandle* out) {
    if (out == nullptr || size == 0) {
        return false;
    }
    std::lock_guard lock(mutex_);
    const auto name_it = name_to_id_.find(name);
    if (name_it == name_to_id_.end()) {
        return false;
    }
    const auto shm_it = shm_domains_.find(name);
    if (shm_it != shm_domains_.end()) {
        return shm_it->second.allocate(shm_it->second.ctx, size, out) == 0;
    }
    const auto dit = domains_.find(name_it->second.low);
    if (dit == domains_.end()) {
        return false;
    }
    return dit->second->allocate(size, out);
}

std::byte* DomainRegistry::resolve_bytes(const substrate::MemoryHandle& handle) {
    std::lock_guard lock(mutex_);
    const auto shm_name_it = shm_id_to_name_.find(handle.domain_id.low);
    if (shm_name_it != shm_id_to_name_.end()) {
        const auto shm_it = shm_domains_.find(shm_name_it->second);
        if (shm_it != shm_domains_.end() && shm_it->second.resolve != nullptr) {
            return reinterpret_cast<std::byte*>(shm_it->second.resolve(shm_it->second.ctx, &handle));
        }
        return nullptr;
    }
    const auto dit = domains_.find(handle.domain_id.low);
    if (dit == domains_.end()) {
        return nullptr;
    }
    return dit->second->resolve_bytes(handle);
}

bool DomainRegistry::deallocate(const substrate::MemoryHandle& handle) {
    std::lock_guard lock(mutex_);
    const auto shm_name_it = shm_id_to_name_.find(handle.domain_id.low);
    if (shm_name_it != shm_id_to_name_.end()) {
        const auto shm_it = shm_domains_.find(shm_name_it->second);
        if (shm_it != shm_domains_.end() && shm_it->second.deallocate != nullptr) {
            return shm_it->second.deallocate(shm_it->second.ctx, &handle) == 0;
        }
        return false;
    }
    const auto dit = domains_.find(handle.domain_id.low);
    if (dit == domains_.end()) {
        return false;
    }
    return dit->second->deallocate(handle);
}

HeapDomain* DomainRegistry::find(const std::string& name) {
    std::lock_guard lock(mutex_);
    const auto it = name_to_id_.find(name);
    if (it == name_to_id_.end()) {
        return nullptr;
    }
    const auto dit = domains_.find(it->second.low);
    return dit == domains_.end() ? nullptr : dit->second.get();
}

HeapDomain* DomainRegistry::find(substrate::DomainId id) {
    std::lock_guard lock(mutex_);
    const auto it = domains_.find(id.low);
    return it == domains_.end() ? nullptr : it->second.get();
}

std::vector<DomainStats> DomainRegistry::stats_all() const {
    std::lock_guard lock(mutex_);
    std::vector<DomainStats> out;
    out.reserve(domains_.size() + shm_domains_.size());
    for (const auto& [_, domain] : domains_) {
        out.push_back(domain->stats());
    }
    for (const auto& [name, _] : shm_domains_) {
        DomainStats s{};
        const auto id_it = name_to_id_.find(name);
        if (id_it != name_to_id_.end()) {
            s.domain_id = id_it->second;
        }
        s.name = name;
        out.push_back(s);
    }
    return out;
}

}  // namespace pymergetic::cruspy::allocator

namespace pymergetic::cruspy::allocator {

void init_allocator_module() {
    DomainRegistry::global().register_heap("heap_default");
}

CRUSPY_NS_MODULE(pymergetic::cruspy::allocator, init_allocator_module);

}  // namespace pymergetic::cruspy::allocator

extern "C" {

int cruspy_allocator_register_heap(const char* name) {
    if (name == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::allocator::DomainRegistry::global().register_heap(name) ? 0 : -1;
}

int cruspy_allocator_register_shm(const char* name, pymergetic::cruspy::allocator::ShmDomainOps ops,
                                  uint64_t* domain_low_out) {
    if (name == nullptr || ops.allocate == nullptr || ops.resolve == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::allocator::DomainRegistry::global().register_shm(name, ops, domain_low_out) ? 0 : -1;
}

int cruspy_allocator_allocate(const char* domain_name, uint64_t size,
                              pymergetic::cruspy::substrate::MemoryHandle* out) {
    if (domain_name == nullptr || out == nullptr || size == 0) {
        return -1;
    }
    return pymergetic::cruspy::allocator::DomainRegistry::global().allocate(domain_name, static_cast<std::size_t>(size),
                                                                             out)
               ? 0
               : -3;
}

int cruspy_allocator_deallocate(const pymergetic::cruspy::substrate::MemoryHandle* handle) {
    if (handle == nullptr || !cruspy_substrate_handle_valid(handle)) {
        return -1;
    }
    return pymergetic::cruspy::allocator::DomainRegistry::global().deallocate(*handle) ? 0 : -3;
}

int cruspy_allocator_stats_json(char* buffer, std::size_t capacity) {
    if (buffer == nullptr || capacity == 0) {
        return -1;
    }
    const auto stats = pymergetic::cruspy::allocator::DomainRegistry::global().stats_all();
    int written = std::snprintf(
        buffer, capacity, "{\"domains\":[");
    if (written < 0 || static_cast<std::size_t>(written) >= capacity) {
        return -2;
    }
    std::size_t pos = static_cast<std::size_t>(written);
    for (std::size_t i = 0; i < stats.size(); ++i) {
        const auto& s = stats[i];
        const int n = std::snprintf(
            buffer + pos, capacity - pos,
            "%s{\"name\":\"%s\",\"bytes_used\":%llu,\"allocation_count\":%llu}",
            i == 0 ? "" : ",",
            s.name.c_str(),
            static_cast<unsigned long long>(s.bytes_used),
            static_cast<unsigned long long>(s.allocation_count));
        if (n < 0 || static_cast<std::size_t>(n) >= capacity - pos) {
            return -2;
        }
        pos += static_cast<std::size_t>(n);
    }
    const int end = std::snprintf(buffer + pos, capacity - pos, "]}");
    if (end < 0 || static_cast<std::size_t>(end) >= capacity - pos) {
        return -2;
    }
    return static_cast<int>(pos + static_cast<std::size_t>(end));
}

}  // extern "C"
