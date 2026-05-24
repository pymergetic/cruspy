#pragma once

namespace pymergetic::cruspy::schema {

/// Compile-time field registry for a model. Declare as `schema_fields` inside each model.
template <auto... MemberPtrs>
struct fields {
  template <typename Model, typename Fn>
  static void for_each(Model& model, Fn&& fn) {
    ((fn(model.*MemberPtrs)), ...);
  }

  template <typename Model, typename Fn>
  static void for_each_const(const Model& model, Fn&& fn) {
    ((fn(model.*MemberPtrs)), ...);
  }
};

}  // namespace pymergetic::cruspy::schema
