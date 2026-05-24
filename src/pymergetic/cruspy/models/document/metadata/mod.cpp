#include "rust/cxx.h"

#include "models/document/metadata/mod.hpp"

namespace pymergetic::cruspy::models::document::metadata {

std::unique_ptr<Metadata> Metadata::make(::rust::String author, std::int32_t tags) {
  return schema::make_model<Metadata>(std::string(author), tags);
}

}  // namespace pymergetic::cruspy::models::document::metadata
