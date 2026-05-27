#include "__init__.hpp"

#include "../../registry/__init__.hpp"

#include <cstdio>
#include <cstring>

namespace pymergetic::cruspy::models::document {

bool document_validate(const substrate::MemoryHandle* handle) {
    if (handle == nullptr) {
        return false;
    }
    int32_t id = 0;
    double score = 0.0;
    if (!registry::field_get_i32(*handle, "id", &id)) {
        return false;
    }
    if (!registry::field_get_f64(*handle, "score", &score)) {
        return false;
    }
    return id >= 0 && id <= 100 && score >= 0.0 && score <= 1.0;
}

void document_normalize(substrate::MemoryHandle* handle) {
    if (handle == nullptr) {
        return;
    }
    int32_t id = 0;
    double score = 0.0;
    registry::field_get_i32(*handle, "id", &id);
    registry::field_get_f64(*handle, "score", &score);
    if (id < 0) {
        id = 0;
    }
    if (score < 0.0) {
        score = 0.0;
    }
    if (score > 1.0) {
        score = 1.0;
    }
    registry::field_set_i32(*handle, "id", id);
    registry::field_set_f64(*handle, "score", score);
}

int document_default_domain(const char* /*fqn*/, char* out, std::size_t capacity) {
    if (out == nullptr || capacity == 0) {
        return -1;
    }
    const char* value = "heap_default";
    std::snprintf(out, capacity, "%s", value);
    return static_cast<int>(std::strlen(value));
}

int document_schema(const char* fqn, char* out, std::size_t capacity) {
    return registry::describe_json(fqn, out, capacity);
}

}  // namespace pymergetic::cruspy::models::document

CRUSPY_REGISTER_METHOD(pymergetic::cruspy::models::document::Document, validate,
                       pymergetic::cruspy::models::document::document_validate)
CRUSPY_REGISTER_METHOD(pymergetic::cruspy::models::document::Document, normalize,
                       pymergetic::cruspy::models::document::document_normalize)
CRUSPY_REGISTER_METHOD(pymergetic::cruspy::models::document::Document, default_domain,
                       pymergetic::cruspy::models::document::document_default_domain)
CRUSPY_REGISTER_METHOD(pymergetic::cruspy::models::document::Document, schema,
                       pymergetic::cruspy::models::document::document_schema)
