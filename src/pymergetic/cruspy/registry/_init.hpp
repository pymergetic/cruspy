#pragma once

#include "../allocator/_init.hpp"
#include "../field/_init.hpp"
#include "../substrate/_init.hpp"

#include <cstddef>
#include <cstdint>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

namespace pymergetic::cruspy::registry {

enum class CType : uint8_t {
    I32,
    F64,
    Object,
};

struct FieldSpec {
    std::string name;
    CType type{CType::I32};
    std::string object_fqn;
    uint32_t offset{};
    uint32_t size{};
    bool has_default{false};
    std::string default_repr;
    bool has_min{false};
    std::string min_repr;
    bool has_max{false};
    std::string max_repr;
    std::string desc;
};

struct TypeEntry {
    std::string fqn;
    uint64_t schema_hash{};
    uint32_t version{1};
    uint32_t size{};
    uint32_t alignment{8};
    std::vector<FieldSpec> fields;
};

class CKlass {
public:
    CKlass(std::string fqn, std::string module_path);
    CKlass& field(const char* name, CType type, const char* object_fqn = nullptr);
    CKlass& field(const field::FieldMeta& meta);
    void register_();

private:
    std::string fqn_;
    std::string module_path_;
    std::vector<FieldSpec> fields_;
};

class TypeRegistry {
public:
    static TypeRegistry& global();

    bool register_type(TypeEntry entry);
    const TypeEntry* lookup(std::string_view fqn) const;
    const TypeEntry* lookup_by_schema_hash(uint64_t schema_hash) const;
    std::vector<std::string> list_fqns() const;

private:
    TypeRegistry() = default;

    mutable std::mutex mutex_;
    std::unordered_map<std::string, TypeEntry> types_;
    std::unordered_map<uint64_t, std::string> hash_to_fqn_;
};

uint64_t compute_schema_hash(const TypeEntry& entry);
TypeEntry build_layout(TypeEntry entry);

bool create_object(std::string_view fqn, std::string_view domain_name, substrate::MemoryHandle* out);
bool field_get_i32(const substrate::MemoryHandle& handle, const char* field, int32_t* out);
bool field_set_i32(const substrate::MemoryHandle& handle, const char* field, int32_t value);
bool field_get_f64(const substrate::MemoryHandle& handle, const char* field, double* out);
bool field_set_f64(const substrate::MemoryHandle& handle, const char* field, double value);
int describe_json(std::string_view fqn, char* buffer, std::size_t capacity);

void bootstrap();

}  // namespace pymergetic::cruspy::registry

#ifdef __cplusplus
extern "C" {
#endif

void cruspy_bootstrap(void);
int cruspy_create(const char* fqn, const char* domain_name, pymergetic::cruspy::substrate::MemoryHandle* out);
int cruspy_field_get_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t* out);
int cruspy_field_set_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t value);
int cruspy_field_get_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double* out);
int cruspy_field_set_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double value);
int cruspy_registry_describe(const char* fqn, char* buffer, std::size_t capacity);

#ifdef __cplusplus
}
#endif
