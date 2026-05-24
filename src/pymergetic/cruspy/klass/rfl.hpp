#pragma once

#include "../module/_init.hpp"
#include "../registry/_init.hpp"
#include "detail.hpp"
#include "meta.hpp"

namespace pymergetic::cruspy::klass {

inline void register_with_registry(const TypeMeta& meta) {
    registry::CKlass builder(meta.fqn, meta.module_path);
    for (const field::FieldMeta& field_meta : meta.fields) {
        builder.field(field_meta);
    }
    builder.register_();
}

template <typename T, auto... MemberPtrs>
  requires(sizeof...(MemberPtrs) > 0)
void register_type() {
    const TypeMeta& meta = Meta<T, MemberPtrs...>::get();
    MetaStore::global().emplace<T>(meta);
    register_with_registry(meta);
}

}  // namespace pymergetic::cruspy::klass

/// Register a klass with the module tree. Use inside the klass's module namespace.
#define CRUSPY_REGISTER_KLASS(Name, ...)                                         \
    namespace {                                                                  \
    void _cruspy_init_##Name() {                                                 \
        ::pymergetic::cruspy::klass::register_type<Name, CRUSPY_FOR_EACH(Name, __VA_ARGS__)>(); \
    }                                                                            \
    [[gnu::used]] ::pymergetic::cruspy::module::ModuleRegistrar                  \
        _cruspy_module_reg_##Name(                                               \
            ::pymergetic::cruspy::klass::Meta<Name, CRUSPY_FOR_EACH(Name, __VA_ARGS__)>::module_path(), \
            _cruspy_init_##Name);                                                \
    }
