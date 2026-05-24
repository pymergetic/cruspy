#pragma once

#include "../../../klass/meta.hpp"
#include "../../../field/_init.hpp"

#include <cstdint>

namespace pymergetic::cruspy::models::document::metadata {

struct Metadata : klass::Klass<Metadata> {
    field::Field<"id", int32_t, field::Attrs<int32_t, "Metadata record id">{0, 0, 100}> id;
};

}  // namespace pymergetic::cruspy::models::document::metadata
