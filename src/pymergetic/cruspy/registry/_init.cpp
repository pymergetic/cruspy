#include "_init.hpp"

#include "../module/_init.hpp"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <functional>
#include <mutex>
#include <string>

namespace pymergetic::cruspy::registry {
namespace {

uint32_t field_size(CType type, const TypeRegistry& registry, const std::string& object_fqn) {
    switch (type) {
        case CType::I32:
            return 4;
        case CType::F64:
            return 8;
        case CType::Object: {
            const auto* nested = registry.lookup(object_fqn);
            return nested == nullptr ? 0 : nested->size;
        }
    }
    return 0;
}

uint32_t align_up(uint32_t value, uint32_t alignment) {
    const uint32_t mask = alignment - 1;
    return (value + mask) & ~mask;
}

substrate::ObjectHeader* object_header(substrate::MemoryHandle& handle) {
    auto* domain = allocator::DomainRegistry::global().find(handle.domain_id);
    if (domain == nullptr) {
        return nullptr;
    }
    auto* bytes = domain->resolve_bytes(handle);
    if (bytes == nullptr) {
        return nullptr;
    }
    return reinterpret_cast<substrate::ObjectHeader*>(bytes);
}

const TypeEntry* entry_for_handle(const substrate::MemoryHandle& handle) {
    if (handle.schema_hash != 0) {
        if (const auto* by_hash = TypeRegistry::global().lookup_by_schema_hash(handle.schema_hash)) {
            return by_hash;
        }
    }
    return TypeRegistry::global().lookup(handle.type_fqn);
}

const FieldSpec* find_field(const TypeEntry& entry, const char* name) {
    if (name == nullptr) {
        return nullptr;
    }
    for (const auto& field : entry.fields) {
        if (field.name == name) {
            return &field;
        }
    }
    return nullptr;
}

}  // namespace

CKlass::CKlass(std::string fqn, std::string module_path)
    : fqn_(std::move(fqn)), module_path_(std::move(module_path)) {}

CKlass& CKlass::field(const char* name, CType type, const char* object_fqn) {
    FieldSpec spec;
    spec.name = name;
    spec.type = type;
    if (object_fqn != nullptr) {
        spec.object_fqn = object_fqn;
    }
    fields_.push_back(std::move(spec));
    return *this;
}

CKlass& CKlass::field(const field::FieldMeta& meta) {
    FieldSpec spec;
    spec.name = meta.name;
    switch (meta.storage) {
        case field::StorageKind::I32:
            spec.type = CType::I32;
            break;
        case field::StorageKind::F64:
            spec.type = CType::F64;
            break;
        case field::StorageKind::Object:
            spec.type = CType::Object;
            spec.object_fqn = meta.object_fqn;
            break;
    }
    spec.has_default = meta.has_default;
    spec.default_repr = meta.default_repr;
    spec.has_min = meta.has_min;
    spec.min_repr = meta.min_repr;
    spec.has_max = meta.has_max;
    spec.max_repr = meta.max_repr;
    spec.desc = meta.desc;
    fields_.push_back(std::move(spec));
    return *this;
}

void CKlass::register_() {
    TypeEntry entry;
    entry.fqn = fqn_;
    entry.fields = fields_;
    entry = build_layout(entry);
    entry.schema_hash = compute_schema_hash(entry);
    TypeRegistry::global().register_type(std::move(entry));
}

TypeRegistry& TypeRegistry::global() {
    static TypeRegistry registry;
    return registry;
}

bool TypeRegistry::register_type(TypeEntry entry) {
    std::lock_guard lock(mutex_);
    if (types_.contains(entry.fqn)) {
        return false;
    }
    hash_to_fqn_.emplace(entry.schema_hash, entry.fqn);
    types_.emplace(entry.fqn, std::move(entry));
    return true;
}

const TypeEntry* TypeRegistry::lookup(std::string_view fqn) const {
    std::lock_guard lock(mutex_);
    const auto it = types_.find(std::string(fqn));
    return it == types_.end() ? nullptr : &it->second;
}

const TypeEntry* TypeRegistry::lookup_by_schema_hash(uint64_t schema_hash) const {
    std::lock_guard lock(mutex_);
    const auto it = hash_to_fqn_.find(schema_hash);
    if (it == hash_to_fqn_.end()) {
        return nullptr;
    }
    const auto tit = types_.find(it->second);
    return tit == types_.end() ? nullptr : &tit->second;
}

std::vector<std::string> TypeRegistry::list_fqns() const {
    std::lock_guard lock(mutex_);
    std::vector<std::string> out;
    out.reserve(types_.size());
    for (const auto& [fqn, _] : types_) {
        out.push_back(fqn);
    }
    std::sort(out.begin(), out.end());
    return out;
}

uint64_t compute_schema_hash(const TypeEntry& entry) {
    std::hash<std::string> hasher;
    uint64_t hash = hasher(entry.fqn);
    for (const auto& field : entry.fields) {
        hash ^= hasher(field.name) + 0x9e3779b97f4a7c15ULL + (hash << 6) + (hash >> 2);
        hash ^= static_cast<uint64_t>(field.type) << 32;
        hash ^= hasher(field.object_fqn);
    }
    return hash;
}

TypeEntry build_layout(TypeEntry entry) {
    const auto& registry = TypeRegistry::global();
    uint32_t offset = align_up(static_cast<uint32_t>(sizeof(substrate::ObjectHeader)), entry.alignment);
    for (auto& field : entry.fields) {
        const uint32_t size = field_size(field.type, registry, field.object_fqn);
        offset = align_up(offset, field.type == CType::F64 ? 8 : 4);
        field.offset = offset;
        field.size = size;
        offset += size;
    }
    entry.size = align_up(offset, entry.alignment);
    return entry;
}

bool create_object(std::string_view fqn, std::string_view domain_name, substrate::MemoryHandle* out) {
    if (out == nullptr) {
        return false;
    }
    const auto* entry = TypeRegistry::global().lookup(fqn);
    if (entry == nullptr) {
        return false;
    }
    if (cruspy_allocator_allocate(domain_name.data(), entry->size, out) != 0) {
        return false;
    }
    out->schema_hash = entry->schema_hash;
    substrate::handle_set_fqn(out, entry->fqn);
    auto* header = object_header(*out);
    if (header == nullptr) {
        return false;
    }
    substrate::header_init(header, entry->schema_hash, entry->version, entry->fqn);
    return true;
}

bool field_get_i32(const substrate::MemoryHandle& handle, const char* field, int32_t* out) {
    if (out == nullptr) {
        return false;
    }
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::I32) {
        return false;
    }
    auto* domain = allocator::DomainRegistry::global().find(handle.domain_id);
    if (domain == nullptr) {
        return false;
    }
    auto* bytes = domain->resolve_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(out, bytes + spec->offset, sizeof(int32_t));
    return true;
}

bool field_set_i32(const substrate::MemoryHandle& handle, const char* field, int32_t value) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::I32) {
        return false;
    }
    auto* domain = allocator::DomainRegistry::global().find(handle.domain_id);
    if (domain == nullptr) {
        return false;
    }
    auto* bytes = domain->resolve_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(bytes + spec->offset, &value, sizeof(int32_t));
    return true;
}

bool field_get_f64(const substrate::MemoryHandle& handle, const char* field, double* out) {
    if (out == nullptr) {
        return false;
    }
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::F64) {
        return false;
    }
    auto* domain = allocator::DomainRegistry::global().find(handle.domain_id);
    if (domain == nullptr) {
        return false;
    }
    auto* bytes = domain->resolve_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(out, bytes + spec->offset, sizeof(double));
    return true;
}

bool field_set_f64(const substrate::MemoryHandle& handle, const char* field, double value) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::F64) {
        return false;
    }
    auto* domain = allocator::DomainRegistry::global().find(handle.domain_id);
    if (domain == nullptr) {
        return false;
    }
    auto* bytes = domain->resolve_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(bytes + spec->offset, &value, sizeof(double));
    return true;
}

int describe_json(std::string_view fqn, char* buffer, std::size_t capacity) {
    const auto* entry = TypeRegistry::global().lookup(fqn);
    if (entry == nullptr || buffer == nullptr || capacity == 0) {
        return -1;
    }
    int written = std::snprintf(
        buffer, capacity,
        "{\"fqn\":\"%s\",\"version\":%u,\"schema_hash\":%llu,\"size\":%u,\"fields\":[",
        entry->fqn.c_str(),
        entry->version,
        static_cast<unsigned long long>(entry->schema_hash),
        entry->size);
    if (written < 0 || static_cast<std::size_t>(written) >= capacity) {
        return -2;
    }
    std::size_t pos = static_cast<std::size_t>(written);
    for (std::size_t i = 0; i < entry->fields.size(); ++i) {
        const auto& f = entry->fields[i];
        const char* type_name = f.type == CType::F64 ? "f64" : (f.type == CType::Object ? "object" : "i32");
        const int n = std::snprintf(
            buffer + pos, capacity - pos,
            "%s{\"name\":\"%s\",\"type\":\"%s\",\"offset\":%u",
            i == 0 ? "" : ",",
            f.name.c_str(),
            type_name,
            f.offset);
        if (n < 0 || static_cast<std::size_t>(n) >= capacity - pos) {
            return -2;
        }
        pos += static_cast<std::size_t>(n);

        auto append_fragment = [&](const char* fragment) -> bool {
            const int m = std::snprintf(buffer + pos, capacity - pos, "%s", fragment);
            if (m < 0 || static_cast<std::size_t>(m) >= capacity - pos) {
                return false;
            }
            pos += static_cast<std::size_t>(m);
            return true;
        };

        if (f.has_default && !f.default_repr.empty()) {
            const std::string default_fragment = ", \"default\":" + f.default_repr;
            if (!append_fragment(default_fragment.c_str())) {
                return -2;
            }
        }
        if (f.has_min && !append_fragment((", \"min\":" + f.min_repr).c_str())) {
            return -2;
        }
        if (f.has_max && !append_fragment((", \"max\":" + f.max_repr).c_str())) {
            return -2;
        }
        if (!f.desc.empty()) {
            const std::string desc_fragment = ", \"desc\":\"" + f.desc + "\"";
            if (!append_fragment(desc_fragment.c_str())) {
                return -2;
            }
        }
        if (!append_fragment("}")) {
            return -2;
        }
    }
    const int end = std::snprintf(buffer + pos, capacity - pos, "]}");
    if (end < 0 || static_cast<std::size_t>(end) >= capacity - pos) {
        return -2;
    }
    return static_cast<int>(pos + static_cast<std::size_t>(end));
}

void bootstrap() { module::ModuleNode::apply_all(); }

}  // namespace pymergetic::cruspy::registry

extern "C" {

void cruspy_bootstrap(void) { pymergetic::cruspy::registry::bootstrap(); }

int cruspy_create(const char* fqn, const char* domain_name, pymergetic::cruspy::substrate::MemoryHandle* out) {
    if (fqn == nullptr || domain_name == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::create_object(fqn, domain_name, out) ? 0 : -2;
}

int cruspy_field_get_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t* out) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_get_i32(*handle, field, out) ? 0 : -2;
}

int cruspy_field_set_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t value) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_set_i32(*handle, field, value) ? 0 : -2;
}

int cruspy_field_get_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double* out) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_get_f64(*handle, field, out) ? 0 : -2;
}

int cruspy_field_set_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double value) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_set_f64(*handle, field, value) ? 0 : -2;
}

int cruspy_registry_describe(const char* fqn, char* buffer, std::size_t capacity) {
    if (fqn == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::describe_json(fqn, buffer, capacity);
}

}  // extern "C"
