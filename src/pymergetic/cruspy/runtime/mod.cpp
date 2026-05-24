#include "core/mod.hpp"

// Runtime entry retained for EP-0010 path compatibility.
const char* pymergetic_cruspy_runtime_version() {
  return pymergetic::cruspy::core::runtime_version();
}
