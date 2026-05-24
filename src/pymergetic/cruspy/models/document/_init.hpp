#pragma once

#include "metadata/_init.hpp"

#include <cstdint>

namespace pymergetic::cruspy::models::document {

struct Document : klass::Klass<Document> {
    field::Field<"id", int32_t, field::Attrs<int32_t, "Primary identifier">{0, 0, 100}> id;
    field::Field<"score", double, field::Attrs<double, "Relevance score">{0.0, 0.0, 1.0}> score;
    field::Field<"meta", metadata::Metadata, field::Attrs<metadata::Metadata, "Nested metadata">{metadata::Metadata{}}>
        meta;
};

}  // namespace pymergetic::cruspy::models::document
