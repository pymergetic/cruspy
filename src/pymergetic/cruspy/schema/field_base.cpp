#include "schema/field_base.hpp"

namespace pymergetic::cruspy::schema {

field_base::~field_base() = default;

bool is_field(const field_base* ptr) noexcept { return ptr != nullptr; }

}  // namespace pymergetic::cruspy::schema
