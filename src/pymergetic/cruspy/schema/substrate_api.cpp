#include "schema/substrate_api.hpp"

#include <cstring>
#include <string_view>

#include "models/document/mod.hpp"

namespace {

enum class FieldTag : std::uint8_t {
  Missing = 0,
  Int = 1,
  Float = 2,
  Bool = 3,
  String = 4,
};

void copy_string(char* dest, std::uint32_t capacity, std::string_view value,
                 std::uint32_t* out_size) {
  const auto len = std::min<std::size_t>(value.size(), capacity > 0 ? capacity - 1 : 0);
  if (capacity > 0 && dest != nullptr) {
    std::memcpy(dest, value.data(), len);
    dest[len] = '\0';
  }
  if (out_size != nullptr) {
    *out_size = static_cast<std::uint32_t>(len);
  }
}

}  // namespace

extern "C" std::int32_t cruspy_schema_encode_document(const void* doc, std::uint8_t* out_data,
                                                      std::uint32_t out_capacity,
                                                      std::uint32_t* out_size) {
  if (doc == nullptr || out_data == nullptr || out_size == nullptr) {
    return -1;
  }
  try {
    const auto& model = *static_cast<const pymergetic::cruspy::models::document::Document*>(doc);
    const auto bytes = model.encode();
    if (bytes.size() > out_capacity) {
      return -2;
    }
    std::memcpy(out_data, bytes.data(), bytes.size());
    *out_size = static_cast<std::uint32_t>(bytes.size());
    return 0;
  } catch (...) {
    return -1;
  }
}

extern "C" std::int32_t cruspy_schema_decode_document_field(
    const std::uint8_t* data, std::uint32_t byte_size, const char* field_name,
    std::uint8_t* out_tag, std::int64_t* out_int, double* out_float, std::uint8_t* out_bool,
    char* out_string, std::uint32_t out_string_capacity, std::uint32_t* out_string_size) {
  if (data == nullptr || field_name == nullptr || out_tag == nullptr) {
    return -1;
  }
  try {
    const auto doc = pymergetic::cruspy::models::document::Document::decode(
        std::span<const std::uint8_t>(data, byte_size));
    const std::string_view name(field_name);
    if (name == "id") {
      *out_tag = static_cast<std::uint8_t>(FieldTag::Int);
      if (out_int != nullptr) {
        *out_int = doc.id.value();
      }
      return 0;
    }
    if (name == "text") {
      *out_tag = static_cast<std::uint8_t>(FieldTag::String);
      copy_string(out_string, out_string_capacity, doc.text.value(), out_string_size);
      return 0;
    }
    if (name == "score") {
      *out_tag = static_cast<std::uint8_t>(FieldTag::Float);
      if (out_float != nullptr) {
        *out_float = doc.score.value();
      }
      return 0;
    }
    if (name == "active") {
      *out_tag = static_cast<std::uint8_t>(FieldTag::Bool);
      if (out_bool != nullptr) {
        *out_bool = doc.active.value() ? 1 : 0;
      }
      return 0;
    }
    if (name == "revision") {
      if (doc.revision.value().has_value()) {
        *out_tag = static_cast<std::uint8_t>(FieldTag::Int);
        if (out_int != nullptr) {
          *out_int = *doc.revision.value();
        }
      } else {
        *out_tag = static_cast<std::uint8_t>(FieldTag::Missing);
      }
      return 0;
    }
    return -2;
  } catch (...) {
    return -1;
  }
}
