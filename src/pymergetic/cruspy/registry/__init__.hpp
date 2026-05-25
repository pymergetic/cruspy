#pragma once

#include "../allocator/__init__.hpp"
#include "../field/__init__.hpp"
#include "../functions/__init__.hpp"
#include "../substrate/__init__.hpp"

#include <cstddef>
#include <cstdint>
#include <functional>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

namespace pymergetic::cruspy::registry {

enum class CType : uint8_t {
    I32,
    I64,
    F64,
    Bool,
    String,
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
    std::unordered_map<std::string, functions::CruspyMethodSlot> methods;
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
    bool register_method(std::string_view fqn, std::string_view name, functions::CruspyMethodSlot slot);
    bool enable_python_method(std::string_view fqn, std::string_view name);
    const TypeEntry* lookup(std::string_view fqn) const;
    const TypeEntry* lookup_by_schema_hash(uint64_t schema_hash) const;
    functions::CruspyMethodSlot* method_slot(std::string_view fqn, std::string_view name);
    bool bind_python_method(std::string_view fqn, std::string_view name, void* py_fn);
    void foreach_python_method(
        const std::function<void(std::string_view fqn, std::string_view name, functions::CruspyMethodSlot& slot)>& fn);
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
bool field_get_i64(const substrate::MemoryHandle& handle, const char* field, int64_t* out);
bool field_set_i64(const substrate::MemoryHandle& handle, const char* field, int64_t value);
bool field_get_f64(const substrate::MemoryHandle& handle, const char* field, double* out);
bool field_set_f64(const substrate::MemoryHandle& handle, const char* field, double value);
bool field_get_bool(const substrate::MemoryHandle& handle, const char* field, bool* out);
bool field_set_bool(const substrate::MemoryHandle& handle, const char* field, bool value);
int field_get_string(const substrate::MemoryHandle& handle, const char* field, char* out, std::size_t capacity);
bool field_set_string(const substrate::MemoryHandle& handle, const char* field, const char* value, std::size_t len);
bool field_get_object(const substrate::MemoryHandle& handle, const char* field, substrate::MemoryHandle* out);
int describe_json(std::string_view fqn, char* buffer, std::size_t capacity);

bool call_bool(const substrate::MemoryHandle& handle, const char* method, bool* out);
bool call_void(substrate::MemoryHandle* handle, const char* method);
bool call_f64(const substrate::MemoryHandle& handle, const char* method, const char* arg0, const char* arg1,
              double* out);
int call_bytes(const substrate::MemoryHandle& handle, const char* method, uint8_t* out, std::size_t capacity);
bool call_constructor(const char* fqn, const char* method, const char* arg0, const char* arg1,
                      substrate::MemoryHandle* out);
int call_static_str(const char* fqn, const char* method, char* out, std::size_t capacity);
int resolve_handle_fqn(const substrate::MemoryHandle& handle, char* out, std::size_t capacity);
bool patch_embedded_schema_hash(const substrate::MemoryHandle& handle, const char* field, uint64_t schema_hash);
void bootstrap();

}  // namespace pymergetic::cruspy::registry

#ifdef __cplusplus
extern "C" {
#endif

void cruspy_bootstrap(void);
int cruspy_create(const char* fqn, const char* domain_name, pymergetic::cruspy::substrate::MemoryHandle* out);
int cruspy_field_get_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t* out);
int cruspy_field_set_i32(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int32_t value);
int cruspy_field_get_i64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int64_t* out);
int cruspy_field_set_i64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int64_t value);
int cruspy_field_get_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double* out);
int cruspy_field_set_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, double value);
int cruspy_field_get_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int* out);
int cruspy_field_set_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, int value);
int cruspy_field_get_string(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field, char* out,
                            std::size_t capacity);
int cruspy_field_set_string(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field,
                            const char* value, std::size_t len);
int cruspy_field_get_object(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* field,
                            pymergetic::cruspy::substrate::MemoryHandle* out);
int cruspy_registry_describe(const char* fqn, char* buffer, std::size_t capacity);
int cruspy_call_bool(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, int* out);
int cruspy_call_void(pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method);
int cruspy_call_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, const char* arg0,
                    const char* arg1, double* out);
int cruspy_call_bytes(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method, uint8_t* out,
                      std::size_t capacity);
int cruspy_call_constructor(const char* fqn, const char* method, const char* arg0, const char* arg1,
                              pymergetic::cruspy::substrate::MemoryHandle* out);
int cruspy_call_static_str(const char* fqn, const char* method, char* out, std::size_t capacity);
int cruspy_register_rust_method(const char* fqn, const char* method, void* rust_fn, int preferred);
int cruspy_register_cpp_method(const char* fqn, const char* method, void* cpp_fn, int preferred);
int cruspy_register_python_method(const char* fqn, const char* method);
int cruspy_bind_python_method(const char* fqn, const char* method, void* py_fn);
void cruspy_foreach_python_method(void (*callback)(const char* fqn, const char* method, void* user), void* user);
int cruspy_resolve_python_methods(void* py_module);
int cruspy_dispatch_python_f64(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method,
                               const char* arg0, const char* arg1, double* out);
int cruspy_dispatch_python_bytes(const pymergetic::cruspy::substrate::MemoryHandle* handle, const char* method,
                                 std::uint8_t* out, std::size_t capacity);
int cruspy_resolve_handle_fqn(const pymergetic::cruspy::substrate::MemoryHandle* handle, char* out,
                              std::size_t capacity);

#ifdef __cplusplus
}
#endif

#define CRUSPY_REGISTER_METHOD(Layout, MethodName, FnPtr)                                                      \
    namespace {                                                                                              \
    [[gnu::constructor]] void _cruspy_reg_method_##MethodName() {                                              \
        ::pymergetic::cruspy::functions::CruspyMethodSlot slot{};                                            \
        slot.cpp_fn = reinterpret_cast<void*>(FnPtr);                                                        \
        slot.available = ::pymergetic::cruspy::functions::kAvailCpp;                                         \
        slot.preferred = ::pymergetic::cruspy::functions::kLangCpp;                                          \
        ::pymergetic::cruspy::registry::TypeRegistry::global().register_method(                                \
            Layout::kFqn, #MethodName, slot);                                                                  \
    }                                                                                                        \
    }
