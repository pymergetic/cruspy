#include "rust/cxx.h"

#include "models/document/mod.hpp"

namespace pymergetic::cruspy::models::document {

std::unique_ptr<Document> Document::make(std::int32_t id, ::rust::String text, double score,
                                         bool active, bool has_revision, std::int32_t revision,
                                         ::rust::String meta_author, std::int32_t meta_tags) {
  return schema::make_model<Document>(id, std::string(text), score, active, has_revision, revision,
                                      std::string(meta_author), meta_tags);
}

}  // namespace pymergetic::cruspy::models::document
