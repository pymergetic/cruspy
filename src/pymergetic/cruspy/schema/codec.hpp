#pragma once

#include <cstdint>
#include <optional>
#include <span>
#include <string>
#include <vector>

#include "schema/types.hpp"

namespace pymergetic::cruspy::schema {

class SchemaWriter {
 public:
  const std::vector<std::uint8_t>& bytes() const { return bytes_; }

  void write_i32(std::int32_t value);
  void write_f64(double value);
  void write_bool(bool value);
  void write_string(std::string_view value);
  void write_optional_i32(const std::optional<std::int32_t>& value);
  void write_bytes(std::span<const std::uint8_t> value);

 private:
  std::vector<std::uint8_t> bytes_;
};

class SchemaReader {
 public:
  explicit SchemaReader(std::span<const std::uint8_t> bytes);

  std::int32_t read_i32();
  double read_f64();
  bool read_bool();
  std::string read_string();
  std::optional<std::int32_t> read_optional_i32();
  std::vector<std::uint8_t> read_bytes();

  std::size_t remaining() const { return bytes_.size() - offset_; }

 private:
  std::span<const std::uint8_t> bytes_;
  std::size_t offset_{0};

  void require(std::size_t count);
};

std::vector<std::uint8_t> encode_fields(std::span<const FieldMeta> fields,
                                        const void* model);
void decode_fields(std::span<const FieldMeta> fields, std::span<const std::uint8_t> bytes,
                   void* model);

}  // namespace pymergetic::cruspy::schema
