#pragma once

#include <cstdint>
#include <string>

#include "model.hpp"

namespace pymergetic::cruspy::models::document {

CRUSPY_MODEL(Document) {
  std::int32_t id;
  std::string text;
  double score;
  bool active;
};

}  // namespace pymergetic::cruspy::models::document
