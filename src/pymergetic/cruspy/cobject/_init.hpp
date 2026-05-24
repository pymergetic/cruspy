#pragma once

#include <cstdint>

namespace pymergetic::cruspy::cobject {

/// Root tag for all cruspy schema entities (klasses, fields, future mixins).
struct CObject {
    static const uint32_t kind_cobject;
};

}  // namespace pymergetic::cruspy::cobject
