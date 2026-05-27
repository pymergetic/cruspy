#include "__init__.hpp"

#include "../module/__init__.hpp"

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <functional>
#include <mutex>
#include <string>
#include <tuple>
#include <vector>

namespace pymergetic::cruspy::registry {
namespace {

std::vector<std::tuple<std::string, std::string, functions::CruspyMethodSlot>>& pending_methods() {
    static std::vector<std::tuple<std::string, std::string, functions::CruspyMethodSlot>> queue;
    return queue;
}

uint32_t field_size(CType type, const TypeRegistry& registry, const std::string& object_fqn) {
    switch (type) {
        case CType::I32:
            return 4;
        case CType::I64:
            return 8;
        case CType::F64:
            return 8;
        case CType::Bool:
            return 1;
        case CType::String:
            return 64;
        case CType::Object: {
            const auto* nested = registry.lookup(object_fqn);
            return nested == nullptr ? 0 : nested->size;
        }
    }
    return 0;
}

uint32_t field_align(CType type) {
    switch (type) {
        case CType::I64:
        case CType::F64:
            return 8;
        default:
            return 4;
    }
}

uint32_t align_up(uint32_t value, uint32_t alignment) {
    const uint32_t mask = alignment - 1;
    return (value + mask) & ~mask;
}

std::byte* object_bytes(const substrate::MemoryHandle& handle) {
    auto* bytes = allocator::DomainRegistry::global().resolve_bytes(handle);
    if (bytes == nullptr) {
        return nullptr;
    }
    return bytes + handle.embedded_offset;
}

substrate::ObjectHeader* object_header(substrate::MemoryHandle& handle) {
    auto* bytes = object_bytes(handle);
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

functions::CruspyMethodSlot* resolve_method_slot(const substrate::MemoryHandle& handle, const char* method) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr || method == nullptr) {
        return nullptr;
    }
    return TypeRegistry::global().method_slot(entry->fqn, method);
}

functions::CruspyMethodSlot* resolve_static_method_slot(const char* fqn, const char* method) {
    if (fqn == nullptr || method == nullptr) {
        return nullptr;
    }
    return TypeRegistry::global().method_slot(fqn, method);
}

extern "C" int cruspy_dispatch_python_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle,
                                          const char* method, const char* arg0, const char* arg1, double* out);
extern "C" int cruspy_dispatch_python_bytes(const pymergetic::cruspy::substrate::MemoryHandle* handle,
                                            const char* method, std::uint8_t* out, std::size_t capacity);

bool dispatch_bool(functions::CruspyMethodSlot* slot, const substrate::MemoryHandle& handle, bool* out) {
    if (slot == nullptr || out == nullptr) {
        return false;
    }
    const uint8_t order[] = {slot->preferred, functions::kLangCpp, functions::kLangRust, functions::kLangPython};
    for (uint8_t lang : order) {
        if ((slot->available & (1u << lang)) == 0) {
            continue;
        }
        if (lang == functions::kLangCpp && slot->cpp_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodBoolFn>(slot->cpp_fn);
            *out = fn(&handle);
            return true;
        }
        if (lang == functions::kLangRust && slot->rust_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodBoolFn>(slot->rust_fn);
            *out = fn(&handle);
            return true;
        }
    }
    return false;
}

bool dispatch_void(functions::CruspyMethodSlot* slot, substrate::MemoryHandle* handle) {
    if (slot == nullptr || handle == nullptr) {
        return false;
    }
    if (slot->cpp_fn != nullptr) {
        auto fn = reinterpret_cast<functions::MethodVoidFn>(slot->cpp_fn);
        fn(handle);
        return true;
    }
    return false;
}

bool dispatch_f64(functions::CruspyMethodSlot* slot, const substrate::MemoryHandle& handle, const char* method,
                  const char* arg0, const char* arg1, double* out) {
    if (slot == nullptr || out == nullptr || method == nullptr) {
        return false;
    }
    const uint8_t order[] = {slot->preferred, functions::kLangCpp, functions::kLangRust, functions::kLangPython};
    for (uint8_t lang : order) {
        if ((slot->available & (1u << lang)) == 0) {
            continue;
        }
        if (lang == functions::kLangRust && slot->rust_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodF64Fn>(slot->rust_fn);
            *out = fn(&handle, arg0, arg1);
            return true;
        }
        if (lang == functions::kLangCpp && slot->cpp_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodF64Fn>(slot->cpp_fn);
            *out = fn(&handle, arg0, arg1);
            return true;
        }
        if (lang == functions::kLangPython && (slot->available & functions::kAvailPython) != 0) {
            if (slot->py_fn != nullptr &&
                cruspy_dispatch_python_f64(&handle, method, arg0, arg1, out) == 0) {
                return true;
            }
            continue;
        }
    }
    return false;
}

int dispatch_bytes(functions::CruspyMethodSlot* slot, const substrate::MemoryHandle& handle, const char* method,
                   std::uint8_t* out, std::size_t capacity) {
    if (slot == nullptr) {
        return -1;
    }
    const uint8_t order[] = {slot->preferred, functions::kLangCpp, functions::kLangRust, functions::kLangPython};
    for (uint8_t lang : order) {
        if ((slot->available & (1u << lang)) == 0) {
            continue;
        }
        if (lang == functions::kLangRust && slot->rust_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodBytesFn>(slot->rust_fn);
            const int rc = fn(&handle, out, capacity);
            if (rc >= 0) {
                return rc;
            }
            continue;
        }
        if (lang == functions::kLangCpp && slot->cpp_fn != nullptr) {
            auto fn = reinterpret_cast<functions::MethodBytesFn>(slot->cpp_fn);
            return fn(&handle, out, capacity);
        }
        if (lang == functions::kLangPython && (slot->available & functions::kAvailPython) != 0) {
            if (slot->py_fn != nullptr) {
                const int rc = cruspy_dispatch_python_bytes(&handle, method, out, capacity);
                if (rc >= 0) {
                    return rc;
                }
            }
        }
    }
    return -1;
}

constexpr std::size_t kSerializeProbeSize = 29;

struct StringSlot {
    uint32_t length;
    char data[60];
};
static_assert(sizeof(StringSlot) == 64);

const char* ctype_name(CType type) {
    switch (type) {
        case CType::I64:
            return "i64";
        case CType::F64:
            return "f64";
        case CType::Bool:
            return "bool";
        case CType::String:
            return "string";
        case CType::Object:
            return "object";
        default:
            return "i32";
    }
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
        case field::StorageKind::I64:
            spec.type = CType::I64;
            break;
        case field::StorageKind::F64:
            spec.type = CType::F64;
            break;
        case field::StorageKind::Bool:
            spec.type = CType::Bool;
            break;
        case field::StorageKind::String:
            spec.type = CType::String;
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

bool TypeRegistry::register_method(std::string_view fqn, std::string_view name, functions::CruspyMethodSlot slot) {
    std::lock_guard lock(mutex_);
    const auto it = types_.find(std::string(fqn));
    if (it == types_.end()) {
        pending_methods().emplace_back(std::string(fqn), std::string(name), slot);
        return true;
    }
    it->second.methods[std::string(name)] = slot;
    return true;
}

bool TypeRegistry::enable_python_method(std::string_view fqn, std::string_view name) {
    std::lock_guard lock(mutex_);
    functions::CruspyMethodSlot slot{};
    slot.available = functions::kAvailPython;
    slot.preferred = functions::kLangPython;
    const auto it = types_.find(std::string(fqn));
    if (it != types_.end()) {
        auto [mit, inserted] = it->second.methods.emplace(std::string(name), slot);
        if (!inserted) {
            mit->second.available |= functions::kAvailPython;
            mit->second.preferred = functions::kLangPython;
        }
        return true;
    }
    pending_methods().emplace_back(std::string(fqn), std::string(name), slot);
    return true;
}

functions::CruspyMethodSlot* TypeRegistry::method_slot(std::string_view fqn, std::string_view name) {
    std::lock_guard lock(mutex_);
    const auto it = types_.find(std::string(fqn));
    if (it == types_.end()) {
        return nullptr;
    }
    const auto mit = it->second.methods.find(std::string(name));
    return mit == it->second.methods.end() ? nullptr : &mit->second;
}

bool TypeRegistry::bind_python_method(std::string_view fqn, std::string_view name, void* py_fn) {
    std::lock_guard lock(mutex_);
    const auto it = types_.find(std::string(fqn));
    if (it == types_.end()) {
        return false;
    }
    const auto mit = it->second.methods.find(std::string(name));
    if (mit == it->second.methods.end()) {
        return false;
    }
    mit->second.py_fn = py_fn;
    return true;
}

void TypeRegistry::foreach_python_method(
    const std::function<void(std::string_view fqn, std::string_view name, functions::CruspyMethodSlot& slot)>& fn) {
    std::lock_guard lock(mutex_);
    for (auto& [fqn, entry] : types_) {
        for (auto& [name, slot] : entry.methods) {
            if ((slot.available & functions::kAvailPython) != 0) {
                fn(fqn, name, slot);
            }
        }
    }
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
        offset = align_up(offset, field_align(field.type));
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
    out->embedded_offset = 0;
    substrate::handle_set_fqn(out, entry->fqn);
    auto* header = object_header(*out);
    if (header == nullptr) {
        return false;
    }
    substrate::header_init(header, entry->schema_hash, entry->version, entry->fqn);
    auto* bytes = object_bytes(*out);
    if (bytes != nullptr) {
        for (const auto& field : entry->fields) {
            if (field.type != CType::Object) {
                continue;
            }
            const auto* nested = TypeRegistry::global().lookup(field.object_fqn);
            if (nested == nullptr) {
                continue;
            }
            auto* nested_header = reinterpret_cast<substrate::ObjectHeader*>(bytes + field.offset);
            substrate::header_init(nested_header, nested->schema_hash, nested->version, nested->fqn);
        }
    }
    return true;
}

namespace {

template <typename T>
bool field_get_scalar(const substrate::MemoryHandle& handle, const char* field, CType expected, T* out) {
    if (out == nullptr) {
        return false;
    }
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != expected) {
        return false;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(out, bytes + spec->offset, sizeof(T));
    return true;
}

template <typename T>
bool field_set_scalar(const substrate::MemoryHandle& handle, const char* field, CType expected, T value) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != expected) {
        return false;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    std::memcpy(bytes + spec->offset, &value, sizeof(T));
    return true;
}

}  // namespace

bool field_get_i32(const substrate::MemoryHandle& handle, const char* field, int32_t* out) {
    return field_get_scalar(handle, field, CType::I32, out);
}

bool field_set_i32(const substrate::MemoryHandle& handle, const char* field, int32_t value) {
    return field_set_scalar(handle, field, CType::I32, value);
}

bool field_get_i64(const substrate::MemoryHandle& handle, const char* field, int64_t* out) {
    return field_get_scalar(handle, field, CType::I64, out);
}

bool field_set_i64(const substrate::MemoryHandle& handle, const char* field, int64_t value) {
    return field_set_scalar(handle, field, CType::I64, value);
}

bool field_get_f64(const substrate::MemoryHandle& handle, const char* field, double* out) {
    return field_get_scalar(handle, field, CType::F64, out);
}

bool field_set_f64(const substrate::MemoryHandle& handle, const char* field, double value) {
    return field_set_scalar(handle, field, CType::F64, value);
}

bool field_get_bool(const substrate::MemoryHandle& handle, const char* field, bool* out) {
    if (out == nullptr) {
        return false;
    }
    uint8_t raw = 0;
    if (!field_get_scalar(handle, field, CType::Bool, &raw)) {
        return false;
    }
    *out = raw != 0;
    return true;
}

bool field_set_bool(const substrate::MemoryHandle& handle, const char* field, bool value) {
    const uint8_t raw = value ? 1 : 0;
    return field_set_scalar(handle, field, CType::Bool, raw);
}

int field_get_string(const substrate::MemoryHandle& handle, const char* field, char* out, std::size_t capacity) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return -1;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::String) {
        return -1;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return -1;
    }
    const auto* slot = reinterpret_cast<const StringSlot*>(bytes + spec->offset);
    const std::size_t length = slot->length > sizeof(slot->data) ? sizeof(slot->data) : slot->length;
    if (capacity == 0) {
        return static_cast<int>(length);
    }
    const std::size_t to_copy = length < capacity - 1 ? length : capacity - 1;
    if (to_copy > 0) {
        std::memcpy(out, slot->data, to_copy);
    }
    out[to_copy] = '\0';
    return static_cast<int>(length);
}

bool field_set_string(const substrate::MemoryHandle& handle, const char* field, const char* value, std::size_t len) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr || value == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::String) {
        return false;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    auto* slot = reinterpret_cast<StringSlot*>(bytes + spec->offset);
    const std::size_t capped = len > sizeof(slot->data) ? sizeof(slot->data) : len;
    slot->length = static_cast<uint32_t>(capped);
    std::memset(slot->data, 0, sizeof(slot->data));
    if (capped > 0) {
        std::memcpy(slot->data, value, capped);
    }
    return true;
}

bool field_get_object(const substrate::MemoryHandle& handle, const char* field, substrate::MemoryHandle* out) {
    if (out == nullptr) {
        return false;
    }
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::Object) {
        return false;
    }
    const auto* nested = TypeRegistry::global().lookup(spec->object_fqn);
    if (nested == nullptr) {
        return false;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    const auto* header = reinterpret_cast<const substrate::ObjectHeader*>(bytes + spec->offset);
    const auto* stored = TypeRegistry::global().lookup_by_schema_hash(header->schema_hash);
    if (stored == nullptr || stored->fqn != nested->fqn) {
        return false;
    }
    *out = handle;
    out->embedded_offset = handle.embedded_offset + spec->offset;
    out->schema_hash = nested->schema_hash;
    out->byte_size = nested->size;
    out->flags |= substrate::kHandleFlagEmbedded;
    substrate::handle_set_fqn(out, nested->fqn);
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
        const int n = std::snprintf(
            buffer + pos, capacity - pos,
            "%s{\"name\":\"%s\",\"type\":\"%s\",\"offset\":%u",
            i == 0 ? "" : ",",
            f.name.c_str(),
            ctype_name(f.type),
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

bool call_bool(const substrate::MemoryHandle& handle, const char* method, bool* out) {
    return dispatch_bool(resolve_method_slot(handle, method), handle, out);
}

bool call_void(substrate::MemoryHandle* handle, const char* method) {
    return dispatch_void(resolve_method_slot(*handle, method), handle);
}

bool call_f64(const substrate::MemoryHandle& handle, const char* method, const char* arg0, const char* arg1,
              double* out) {
    return dispatch_f64(resolve_method_slot(handle, method), handle, method, arg0, arg1, out);
}

int call_bytes(const substrate::MemoryHandle& handle, const char* method, uint8_t* out, std::size_t capacity) {
    auto* slot = resolve_method_slot(handle, method);
    if (slot == nullptr) {
        return -1;
    }
    return dispatch_bytes(slot, handle, method, out, capacity);
}

bool call_constructor(const char* fqn, const char* method, const char* arg0, const char* arg1,
                      substrate::MemoryHandle* out) {
    if (out == nullptr) {
        return false;
    }
    auto* slot = resolve_static_method_slot(fqn, method);
    if (slot == nullptr || slot->rust_fn == nullptr) {
        return false;
    }
    auto fn = reinterpret_cast<functions::MethodConstructorFn>(slot->rust_fn);
    return fn(fqn, out, arg0, arg1) == 0;
}

int call_static_str(const char* fqn, const char* method, char* out, std::size_t capacity) {
    auto* slot = resolve_static_method_slot(fqn, method);
    if (slot == nullptr || slot->cpp_fn == nullptr) {
        return -1;
    }
    auto fn = reinterpret_cast<functions::MethodStaticStrFn>(slot->cpp_fn);
    return fn(fqn, out, capacity);
}

int resolve_handle_fqn(const substrate::MemoryHandle& handle, char* out, std::size_t capacity) {
    if (out == nullptr || capacity == 0) {
        return -1;
    }
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return -2;
    }
    const int written = std::snprintf(out, capacity, "%s", entry->fqn.c_str());
    if (written < 0 || static_cast<std::size_t>(written) >= capacity) {
        return -3;
    }
    return written;
}

bool patch_embedded_schema_hash(const substrate::MemoryHandle& handle, const char* field, uint64_t schema_hash) {
    const auto* entry = entry_for_handle(handle);
    if (entry == nullptr) {
        return false;
    }
    const auto* spec = find_field(*entry, field);
    if (spec == nullptr || spec->type != CType::Object) {
        return false;
    }
    auto* bytes = object_bytes(handle);
    if (bytes == nullptr) {
        return false;
    }
    auto* header = reinterpret_cast<substrate::ObjectHeader*>(bytes + spec->offset);
    header->schema_hash = schema_hash;
    return true;
}

void bootstrap() {
    module::ModuleNode::apply_all();
    std::vector<std::tuple<std::string, std::string, functions::CruspyMethodSlot>> pending;
    pending.swap(pending_methods());
    for (auto& [fqn, name, slot] : pending) {
        TypeRegistry::global().register_method(fqn, name, slot);
    }
}

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

int cruspy_field_get_i64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int64_t* out) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_get_i64(*handle, field, out) ? 0 : -2;
}

int cruspy_field_set_i64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int64_t value) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_set_i64(*handle, field, value) ? 0 : -2;
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

int cruspy_field_get_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int* out) {
    if (handle == nullptr || out == nullptr) {
        return -1;
    }
    bool value = false;
    if (!pymergetic::cruspy::registry::field_get_bool(*handle, field, &value)) {
        return -2;
    }
    *out = value ? 1 : 0;
    return 0;
}

int cruspy_field_set_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int value) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_set_bool(*handle, field, value != 0) ? 0 : -2;
}

int cruspy_field_get_string(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, char* out,
                            std::size_t capacity) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_get_string(*handle, field, out, capacity);
}

int cruspy_field_set_string(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field,
                            const char* value, std::size_t len) {
    if (handle == nullptr || value == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_set_string(*handle, field, value, len) ? 0 : -2;
}

int cruspy_field_get_object(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field,
                            pymergetic::cruspy::substrate::MemoryHandle* out) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::field_get_object(*handle, field, out) ? 0 : -2;
}

int cruspy_registry_describe(const char* fqn, char* buffer, std::size_t capacity) {
    if (fqn == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::describe_json(fqn, buffer, capacity);
}

int cruspy_call_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, int* out) {
    if (handle == nullptr || out == nullptr) {
        return -1;
    }
    bool value = false;
    if (!pymergetic::cruspy::registry::call_bool(*handle, method, &value)) {
        return -2;
    }
    *out = value ? 1 : 0;
    return 0;
}

int cruspy_call_void(pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::call_void(handle, method) ? 0 : -2;
}

int cruspy_call_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, const char* arg0,
                    const char* arg1, double* out) {
    if (handle == nullptr || out == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::call_f64(*handle, method, arg0, arg1, out) ? 0 : -2;
}

int cruspy_call_bytes(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, uint8_t* out,
                      std::size_t capacity) {
    if (handle == nullptr) {
        return -1;
    }
    if (capacity == 0) {
        return pymergetic::cruspy::registry::call_bytes(*handle, method, nullptr, 0);
    }
    return pymergetic::cruspy::registry::call_bytes(*handle, method, out, capacity);
}

int cruspy_call_constructor(const char* fqn, const char* method, const char* arg0, const char* arg1,
                            pymergetic::cruspy::substrate::MemoryHandle* out) {
    if (fqn == nullptr || method == nullptr || out == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::call_constructor(fqn, method, arg0, arg1, out) ? 0 : -2;
}

int cruspy_call_static_str(const char* fqn, const char* method, char* out, std::size_t capacity) {
    if (fqn == nullptr || method == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::call_static_str(fqn, method, out, capacity);
}

int cruspy_register_rust_method(const char* fqn, const char* method, void* rust_fn, int preferred) {
    if (fqn == nullptr || method == nullptr || rust_fn == nullptr) {
        return -1;
    }
    pymergetic::cruspy::functions::CruspyMethodSlot slot{};
    slot.rust_fn = rust_fn;
    slot.available = pymergetic::cruspy::functions::kAvailRust;
    slot.preferred = static_cast<uint8_t>(preferred);
    return pymergetic::cruspy::registry::TypeRegistry::global().register_method(fqn, method, slot) ? 0 : -2;
}

int cruspy_register_cpp_method(const char* fqn, const char* method, void* cpp_fn, int preferred) {
    if (fqn == nullptr || method == nullptr || cpp_fn == nullptr) {
        return -1;
    }
    pymergetic::cruspy::functions::CruspyMethodSlot slot{};
    slot.cpp_fn = cpp_fn;
    slot.available = pymergetic::cruspy::functions::kAvailCpp;
    slot.preferred = static_cast<uint8_t>(preferred);
    return pymergetic::cruspy::registry::TypeRegistry::global().register_method(fqn, method, slot) ? 0 : -2;
}

int cruspy_register_python_method(const char* fqn, const char* method) {
    if (fqn == nullptr || method == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::TypeRegistry::global().enable_python_method(fqn, method) ? 0 : -2;
}

int cruspy_bind_python_method(const char* fqn, const char* method, void* py_fn) {
    if (fqn == nullptr || method == nullptr || py_fn == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::TypeRegistry::global().bind_python_method(fqn, method, py_fn) ? 0 : -2;
}

void cruspy_foreach_python_method(void (*callback)(const char* fqn, const char* method, void* user), void* user) {
    if (callback == nullptr) {
        return;
    }
    pymergetic::cruspy::registry::TypeRegistry::global().foreach_python_method(
        [callback, user](std::string_view fqn, std::string_view name,
                           pymergetic::cruspy::functions::CruspyMethodSlot& slot) {
            if (slot.py_fn == nullptr) {
                callback(fqn.data(), name.data(), user);
            }
        });
}

int cruspy_test_patch_field_schema_hash(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field,
                                        uint64_t schema_hash) {
    if (handle == nullptr || field == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::patch_embedded_schema_hash(*handle, field, schema_hash) ? 0 : -2;
}

int cruspy_resolve_handle_fqn(const pymergetic::cruspy::substrate::MemoryHandle* handle, char* out,
                              std::size_t capacity) {
    if (handle == nullptr) {
        return -1;
    }
    return pymergetic::cruspy::registry::resolve_handle_fqn(*handle, out, capacity);
}

}  // extern "C"
