#include "functions/mod.hpp"

#include "errors/mod.hpp"

namespace pymergetic::cruspy::functions {

TransformSlot& transform_slot() {
  static TransformSlot slot;
  return slot;
}

void set_transform(float (*trampoline)(void*, float), void* context) {
  auto& slot = transform_slot();
  slot.trampoline = trampoline;
  slot.context = context;
}

float call_transform(float value) {
  const auto& slot = transform_slot();
  if (!slot.is_set()) {
    throw BridgeError("BridgeError: transform slot is not registered");
  }
  return slot.call(value);
}

}  // namespace pymergetic::cruspy::functions
