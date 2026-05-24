#include "pymergetic-cruspy/generated/models/token/mod.rs.h"

#include "errors/mod.hpp"
#include "models/token/mod.hpp"

namespace pymergetic::cruspy::models::token {

void validate_token(const Token& token) {
  if (token.id < 1) {
    throw ValidationError("cruspy.validation:id must be >= 1");
  }
  if (token.score < 0.0 || token.score > 1.0) {
    throw ValidationError("cruspy.validation:score must be between 0.0 and 1.0");
  }
  if (token.value.size() > 512) {
    throw ValidationError("cruspy.validation:value exceeds maximum length of 512");
  }
}

}  // namespace pymergetic::cruspy::models::token
