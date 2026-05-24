#include "runtime/mod.hpp"

#ifndef CRUSPY_PKG_VERSION
#define CRUSPY_PKG_VERSION "unknown"
#endif

namespace pymergetic::cruspy {

const char* runtime_version() { return CRUSPY_PKG_VERSION; }

}  // namespace pymergetic::cruspy

extern "C" const char* cruspy_runtime_version() {
  return pymergetic::cruspy::runtime_version();
}
