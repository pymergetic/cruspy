#include "schema/schema_base.hpp"

namespace pymergetic::cruspy::schema {

schema_base::~schema_base() = default;

bool is_schema(const schema_base* ptr) noexcept { return ptr != nullptr; }

}  // namespace pymergetic::cruspy::schema
