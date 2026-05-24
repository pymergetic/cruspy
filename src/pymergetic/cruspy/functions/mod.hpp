#pragma once

#include <cstdint>
#include <functional>

namespace pymergetic::cruspy::functions {

template <typename Ret, typename... Args>
struct FunctionSlot {
  void* context{nullptr};
  Ret (*trampoline)(void*, Args...) {nullptr};

  Ret call(Args... args) const {
    return trampoline(context, args...);
  }

  bool is_set() const { return trampoline != nullptr; }
};

using TransformSlot = FunctionSlot<float, float>;

TransformSlot& transform_slot();

void set_transform(float (*trampoline)(void*, float), void* context);

float call_transform(float value);

}  // namespace pymergetic::cruspy::functions
