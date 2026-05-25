#pragma once

#include "../cobject/__init__.hpp"

#include <rfl/internal/StringLiteral.hpp>

#include <cstdint>
#include <string>
#include <type_traits>
#include <utility>

namespace pymergetic::cruspy::field {

/// Tag type for reflected string fields (storage lives in the registry string slot).
struct StringTag {};

enum class StorageKind : uint8_t {
    I32,
    I64,
    F64,
    Bool,
    String,
    Object,
};

struct FieldMeta {
    std::string name;
    StorageKind storage{StorageKind::I32};
    std::string object_fqn;
    bool has_default{false};
    std::string default_repr;
    bool has_min{false};
    std::string min_repr;
    bool has_max{false};
    std::string max_repr;
    std::string desc;
};

/// Compile-time field attributes (default, bounds, description).
template <typename Value, rfl::internal::StringLiteral Desc = "">
struct Attrs {
    Value default_value{};
    bool has_default{false};
    bool has_min{false};
    Value min{};
    bool has_max{false};
    Value max{};

    static constexpr auto description = Desc;

    consteval Attrs() = default;

    consteval Attrs(Value default_value) : default_value(default_value), has_default(true) {}

    consteval Attrs(Value default_value, Value min_value, Value max_value)
        : default_value(default_value),
          has_default(true),
          has_min(true),
          min(min_value),
          has_max(true),
          max(max_value) {}
};

namespace detail {

inline std::string repr(int32_t value) { return std::to_string(value); }
inline std::string repr(int64_t value) { return std::to_string(value); }
inline std::string repr(double value) { return std::to_string(value); }
inline std::string repr(float value) { return std::to_string(value); }
inline std::string repr(bool value) { return value ? "true" : "false"; }

template <typename Value>
inline std::string repr(const Value&) {
    return {};
}

template <typename AttrsType>
inline void apply_attrs(FieldMeta& meta, const AttrsType& attrs) {
    if (attrs.has_default) {
        meta.has_default = true;
        meta.default_repr = repr(attrs.default_value);
    }
    if (attrs.has_min) {
        meta.has_min = true;
        meta.min_repr = repr(attrs.min);
    }
    if (attrs.has_max) {
        meta.has_max = true;
        meta.max_repr = repr(attrs.max);
    }
    if constexpr (AttrsType::description.length > 0) {
        meta.desc = std::string(AttrsType::description.string_view());
    }
}

}  // namespace detail

/// Tag for reflected field members (metadata lives in ``klass::Meta`` / ``FieldMeta``).
struct FieldBase : cobject::CObject {
    static constexpr uint32_t kind_field = 2;
};

template <typename T>
struct IsCruspyField : std::false_type {};

/// Cruspy field member — value storage plus compile-time name and optional attrs.
template <rfl::internal::StringLiteral Name, typename Value, auto Spec = Attrs<Value>{}>
struct Field : FieldBase {
    using Type = Value;

    static constexpr auto attrs = Spec;

    Value value{Spec.has_default ? Spec.default_value : Value{}};

    constexpr Field() = default;

    constexpr Field(const Value& v) : value(v) {}
    constexpr Field(Value&& v) noexcept(std::is_nothrow_move_constructible_v<Value>) : value(std::move(v)) {}

    static constexpr auto name() { return Name; }

    constexpr operator Value&() { return value; }
    constexpr operator const Value&() const { return value; }

    constexpr Field& operator=(const Value& v) {
        value = v;
        return *this;
    }

    constexpr Field& operator=(Value&& v) noexcept(std::is_nothrow_move_assignable_v<Value>) {
        value = std::move(v);
        return *this;
    }
};

template <rfl::internal::StringLiteral Name, typename Value, auto Spec>
struct IsCruspyField<Field<Name, Value, Spec>> : std::true_type {};

}  // namespace pymergetic::cruspy::field
