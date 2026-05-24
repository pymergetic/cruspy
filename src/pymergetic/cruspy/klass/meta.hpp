#pragma once

#include "__init__.hpp"
#include "detail.hpp"

#include <cstdint>
#include <string>
#include <type_traits>
#include <utility>

namespace pymergetic::cruspy::klass {

namespace detail {

template <auto MemberPtr>
struct FieldMember;

template <typename Klass, typename FieldType, FieldType Klass::*Member>
  requires field::IsCruspyField<FieldType>::value
struct FieldMember<Member> {
    using klass_type = Klass;
    using field_type = FieldType;
    using value_type = typename FieldType::Type;

    static const char* name() {
        static constexpr auto literal = FieldType::name();
        return literal.arr_.data();
    }
};

template <typename T>
inline constexpr bool is_object_storage_v =
    !std::is_same_v<std::remove_cvref_t<T>, int32_t> && !std::is_same_v<std::remove_cvref_t<T>, int64_t> &&
    !std::is_same_v<std::remove_cvref_t<T>, double> && !std::is_same_v<std::remove_cvref_t<T>, float> &&
    !std::is_same_v<std::remove_cvref_t<T>, bool>;

template <typename T>
struct storage_kind_of {
    static_assert(sizeof(T) == 0, "unsupported cruspy field value type");
};

template <>
struct storage_kind_of<int32_t> {
    static constexpr field::StorageKind value = field::StorageKind::I32;
};
template <>
struct storage_kind_of<int64_t> {
    static constexpr field::StorageKind value = field::StorageKind::I64;
};
template <>
struct storage_kind_of<double> {
    static constexpr field::StorageKind value = field::StorageKind::F64;
};
template <>
struct storage_kind_of<float> {
    static constexpr field::StorageKind value = field::StorageKind::F64;
};
template <>
struct storage_kind_of<bool> {
    static constexpr field::StorageKind value = field::StorageKind::Bool;
};

template <typename T>
  requires is_object_storage_v<T>
struct storage_kind_of<T> {
    static constexpr field::StorageKind value = field::StorageKind::Object;
};

template <auto MemberPtr>
void append_field_meta(TypeMeta& meta) {
    using Binding = FieldMember<MemberPtr>;
    using Value = typename Binding::value_type;

    field::FieldMeta field_meta;
    field_meta.name = Binding::name();
    field_meta.storage = storage_kind_of<Value>::value;
    if constexpr (storage_kind_of<Value>::value == field::StorageKind::Object) {
        field_meta.object_fqn = type_fqn<Value>();
    }
    field::detail::apply_attrs(field_meta, Binding::field_type::attrs);
    meta.fields.push_back(std::move(field_meta));
}

}  // namespace detail

template <typename T, auto... MemberPtrs>
  requires(sizeof...(MemberPtrs) > 0)
struct Meta {
    using Type = std::remove_cvref_t<T>;

    static const TypeMeta& get() {
        static const TypeMeta cached = build();
        return cached;
    }

    static TypeMeta build() {
        TypeMeta meta;
        meta.module_path = module_path();
        meta.type_name = type_name();
        meta.fqn = meta.make_fqn();
        (detail::append_field_meta<MemberPtrs>(meta), ...);
        return meta;
    }

    static const char* module_path() {
        static constexpr auto path = detail::type_module_path_array<Type>();
        return path.data();
    }

    static const char* type_name() {
        static constexpr auto name = detail::type_short_name_array<Type>();
        return name.data();
    }
};

template <typename T>
const TypeMeta& meta_of() {
    if (const TypeMeta* stored = MetaStore::global().find<T>()) {
        return *stored;
    }
    static const TypeMeta empty;
    return empty;
}

/// External metadata accessor for a klass type (data lives in ``Meta``, not in the struct).
template <typename Derived>
struct Klass : KlassBase {
    using type = Derived;

    [[nodiscard]] static const TypeMeta& meta() { return meta_of<Derived>(); }
};

template <typename T>
struct IsCruspyKlass : std::bool_constant<std::is_base_of_v<KlassBase, T>> {};

}  // namespace pymergetic::cruspy::klass
