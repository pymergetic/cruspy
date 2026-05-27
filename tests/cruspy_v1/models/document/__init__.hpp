#pragma once

#include "__init___gen.hpp"  // IWYU pragma: export
#include "../../substrate/__init__.hpp"

#include <cstddef>

namespace pymergetic::cruspy::models::document {

bool document_validate(const substrate::MemoryHandle* handle);
void document_normalize(substrate::MemoryHandle* handle);
int document_default_domain(const char* fqn, char* out, std::size_t capacity);
int document_schema(const char* fqn, char* out, std::size_t capacity);

}  // namespace pymergetic::cruspy::models::document
