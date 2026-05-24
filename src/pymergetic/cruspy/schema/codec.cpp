#include "schema/codec.hpp"

#include <cstring>
#include <optional>
#include <string>
#include <vector>

#include "errors/mod.hpp"

namespace pymergetic::cruspy::schema {

namespace {

void append_u32(std::vector<std::uint8_t>& out, std::uint32_t value) {
  out.push_back(static_cast<std::uint8_t>(value & 0xFF));
  out.push_back(static_cast<std::uint8_t>((value >> 8) & 0xFF));
  out.push_back(static_cast<std::uint8_t>((value >> 16) & 0xFF));
  out.push_back(static_cast<std::uint8_t>((value >> 24) & 0xFF));
}

std::uint32_t read_u32(std::span<const std::uint8_t> bytes, std::size_t& offset) {
  if (offset + 4 > bytes.size()) {
    throw ShmError("cruspy.shm: schema decode underflow");
  }
  std::uint32_t value = 0;
  std::memcpy(&value, bytes.data() + offset, 4);
  offset += 4;
  return value;
}

}  // namespace

void SchemaWriter::write_i32(std::int32_t value) {
  append_u32(bytes_, static_cast<std::uint32_t>(value));
}

void SchemaWriter::write_f64(double value) {
  static_assert(sizeof(double) == 8);
  const auto* raw = reinterpret_cast<const std::uint8_t*>(&value);
  bytes_.insert(bytes_.end(), raw, raw + 8);
}

void SchemaWriter::write_bool(bool value) {
  bytes_.push_back(value ? 1 : 0);
}

void SchemaWriter::write_string(std::string_view value) {
  append_u32(bytes_, static_cast<std::uint32_t>(value.size()));
  bytes_.insert(bytes_.end(), value.begin(), value.end());
}

void SchemaWriter::write_optional_i32(const std::optional<std::int32_t>& value) {
  if (value.has_value()) {
    write_bool(true);
    write_i32(*value);
  } else {
    write_bool(false);
    write_i32(0);
  }
}

void SchemaWriter::write_bytes(std::span<const std::uint8_t> value) {
  append_u32(bytes_, static_cast<std::uint32_t>(value.size()));
  bytes_.insert(bytes_.end(), value.begin(), value.end());
}

SchemaReader::SchemaReader(std::span<const std::uint8_t> bytes) : bytes_(bytes) {}

void SchemaReader::require(std::size_t count) {
  if (offset_ + count > bytes_.size()) {
    throw ShmError("cruspy.shm: schema decode underflow");
  }
}

std::int32_t SchemaReader::read_i32() {
  return static_cast<std::int32_t>(read_u32(bytes_, offset_));
}

double SchemaReader::read_f64() {
  require(8);
  double value = 0.0;
  std::memcpy(&value, bytes_.data() + offset_, 8);
  offset_ += 8;
  return value;
}

bool SchemaReader::read_bool() {
  require(1);
  return bytes_[offset_++] != 0;
}

std::string SchemaReader::read_string() {
  const auto len = read_u32(bytes_, offset_);
  require(len);
  std::string value(reinterpret_cast<const char*>(bytes_.data() + offset_), len);
  offset_ += len;
  return value;
}

std::optional<std::int32_t> SchemaReader::read_optional_i32() {
  const bool present = read_bool();
  const auto value = read_i32();
  return present ? std::optional<std::int32_t>{value} : std::nullopt;
}

std::vector<std::uint8_t> SchemaReader::read_bytes() {
  const auto len = read_u32(bytes_, offset_);
  require(len);
  std::vector<std::uint8_t> value(bytes_.begin() + static_cast<std::ptrdiff_t>(offset_),
                                    bytes_.begin() + static_cast<std::ptrdiff_t>(offset_ + len));
  offset_ += len;
  return value;
}

std::vector<std::uint8_t> encode_fields(std::span<const FieldMeta> fields,
                                        const void* model) {
  (void)fields;
  (void)model;
  throw ShmError("cruspy.shm: generic encode_fields not implemented");
}

void decode_fields(std::span<const FieldMeta> fields, std::span<const std::uint8_t> bytes,
                   void* model) {
  (void)fields;
  (void)model;
  (void)bytes;
  throw ShmError("cruspy.shm: generic decode_fields not implemented");
}

}  // namespace pymergetic::cruspy::schema
