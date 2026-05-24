#pragma once

// cruspy:models

#include <cstdint>
#include <memory>
#include <optional>
#include <string>

#include "models/document/metadata/mod.hpp"
#include "schema/annotations.hpp"
#include "schema/field.hpp"
#include "schema/fields.hpp"
#include "schema/model.hpp"

#include "rust/cxx.h"

namespace pymergetic::cruspy::models::document {

using metadata::Metadata;

struct Document : schema::model<
    Document,
    "Document",
    schema::desc<"Indexed text document">> {

  schema::field<
      Document,
      "id",
      std::int32_t,
      schema::min<1>,
      schema::desc<"Primary key">>
      id{};

  schema::field<
      Document,
      "text",
      std::string,
      schema::max_len<512>,
      schema::desc<"Body text">>
      text{};

  schema::field<
      Document,
      "score",
      double,
      schema::ge<0.0>,
      schema::le<1.0>,
      schema::desc<"Relevance score">>
      score{};

  schema::field<
      Document,
      "active",
      bool,
      schema::desc<"Whether the document is active">>
      active{false};

  schema::field<
      Document,
      "revision",
      std::optional<std::int32_t>,
      schema::min<0>,
      schema::desc<"Content revision, if known">>
      revision{};

  schema::field<
      Document,
      "meta",
      Metadata,
      schema::desc<"Document metadata">>
      meta{};

  using schema_fields = schema::fields<
      &Document::id,
      &Document::text,
      &Document::score,
      &Document::active,
      &Document::revision,
      &Document::meta>;

  static std::unique_ptr<Document> make(std::int32_t id, ::rust::String text, double score,
                                        bool active, bool has_revision, std::int32_t revision,
                                        ::rust::String meta_author, std::int32_t meta_tags);
};

}  // namespace pymergetic::cruspy::models::document
