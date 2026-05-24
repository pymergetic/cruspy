#pragma once

#include <cstdint>
#include <string>

#include "model.hpp"

namespace pymergetic::cruspy::models::token {

CRUSPY_MODEL(Token) {
  std::int32_t id;       // CRUSPY_MIN(1)
  std::string value;     // CRUSPY_MAX_LEN(512)
  double score;          // CRUSPY_GE(0.0) CRUSPY_LE(1.0)
  bool active;
};

}  // namespace pymergetic::cruspy::models::token
