#pragma once

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <type_traits>
#include <vector>

#include "schema/codec.hpp"
#include "schema/field.hpp"
#include "schema/model.hpp"
#include "schema/validate.hpp"

namespace pymergetic::cruspy::schema {

namespace detail {

inline void encode_value(SchemaWriter& writer, std::int32_t value) { writer.write_i32(value); }

inline void encode_value(SchemaWriter& writer, double value) { writer.write_f64(value); }

inline void encode_value(SchemaWriter& writer, bool value) { writer.write_bool(value); }

inline void encode_value(SchemaWriter& writer, const std::string& value) {
  writer.write_string(value);
}

inline void encode_value(SchemaWriter& writer, const std::optional<std::int32_t>& value) {
  writer.write_optional_i32(value);
}

template <typename T>
  requires is_model_type_v<T>
void encode_value(SchemaWriter& writer, const T& model) {
  writer.write_bytes(encode_model(model, false));
}

inline std::int32_t decode_value(SchemaReader& reader, std::type_identity<std::int32_t>) {
  return reader.read_i32();
}

inline double decode_value(SchemaReader& reader, std::type_identity<double>) {
  return reader.read_f64();
}

inline bool decode_value(SchemaReader& reader, std::type_identity<bool>) { return reader.read_bool(); }

inline std::string decode_value(SchemaReader& reader, std::type_identity<std::string>) {
  return reader.read_string();
}

inline std::optional<std::int32_t> decode_value(SchemaReader& reader,
                                                std::type_identity<std::optional<std::int32_t>>) {
  return reader.read_optional_i32();
}

template <typename T>
  requires is_model_type_v<T>
T decode_value(SchemaReader& reader, std::type_identity<T>) {
  return decode_model<T>(std::span<const std::uint8_t>(reader.read_bytes()), false);
}

}  // namespace detail

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
void encode_field(SchemaWriter& writer,
                  const field<Model, Name, T, Annotations...>& member) {
  detail::encode_value(writer, member.value());
}

template <typename Model, rfl::internal::StringLiteral Name, typename T, typename... Annotations>
void decode_field(SchemaReader& reader, field<Model, Name, T, Annotations...>& member) {
  member = detail::decode_value(reader, std::type_identity<T>{});
}

template <typename T>
  requires is_model_type_v<T>
std::vector<std::uint8_t> encode_model(const T& model, bool validate) {
  if (validate) {
    validate_model(model);
  }
  SchemaWriter writer;
  model.for_each_field([&](const auto& member) { encode_field(writer, member); });
  return writer.bytes();
}

template <typename T>
  requires is_model_type_v<T>
T decode_model(std::span<const std::uint8_t> bytes, bool validate) {
  SchemaReader reader(bytes);
  T model{};
  model.for_each_field([&](auto& member) { decode_field(reader, member); });
  if (validate) {
    validate_model(model);
  }
  return model;
}

}  // namespace pymergetic::cruspy::schema
