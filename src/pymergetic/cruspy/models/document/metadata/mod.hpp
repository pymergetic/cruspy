#pragma once

// cruspy:models

#include <cstdint>
#include <memory>
#include <string>

#include "schema/annotations.hpp"
#include "schema/field.hpp"
#include "schema/fields.hpp"
#include "schema/model.hpp"

#include "rust/cxx.h"

namespace pymergetic::cruspy::models::document::metadata {

struct Metadata : schema::model<
    Metadata,
    "Metadata",
    schema::desc<"Document metadata">> {

  schema::field<
      Metadata,
      "author",
      std::string,
      schema::max_len<64>,
      schema::desc<"Author name">>
      author{};

  schema::field<
      Metadata,
      "tags",
      std::int32_t,
      schema::min<0>,
      schema::desc<"Tag count">>
      tags{0};

  using schema_fields = schema::fields<
      &Metadata::author,
      &Metadata::tags>;

  static std::unique_ptr<Metadata> make(::rust::String author, std::int32_t tags);
};

}  // namespace pymergetic::cruspy::models::document::metadata
