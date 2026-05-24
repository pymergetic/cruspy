#include "schema/model_base.hpp"

namespace pymergetic::cruspy::schema {

model_base::~model_base() = default;

bool is_model(const model_base* ptr) noexcept { return ptr != nullptr; }

}  // namespace pymergetic::cruspy::schema
