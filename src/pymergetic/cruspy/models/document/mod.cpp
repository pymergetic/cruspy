#include "pymergetic-cruspy/generated/models/document/mod.rs.h"

#include "errors/mod.hpp"
#include "models/document/mod.hpp"

namespace pymergetic::cruspy::models::document {

void validate_document(const Document& doc) {
  if (doc.id < 1) {
    throw ValidationError("ValidationError: id must be >= 1");
  }
  if (doc.score < 0.0 || doc.score > 1.0) {
    throw ValidationError("ValidationError: score must be between 0.0 and 1.0");
  }
  if (doc.text.size() > 512) {
    throw ValidationError("ValidationError: text exceeds maximum length of 512");
  }
}

}  // namespace pymergetic::cruspy::models::document
