#pragma once

/// Backward-compatible include — klass registration lives in ``klass/rfl.hpp``.
#include "../klass/rfl.hpp"

namespace pymergetic::cruspy::registry {

template <typename T, auto... MemberPtrs>
void register_rfl() {
    klass::register_type<T, MemberPtrs...>();
}

}  // namespace pymergetic::cruspy::registry
