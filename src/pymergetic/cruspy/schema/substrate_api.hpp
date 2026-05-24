#pragma once

#include <cstdint>

extern "C" {

std::int32_t cruspy_schema_encode_document(const void* doc, std::uint8_t* out_data,
                                           std::uint32_t out_capacity,
                                           std::uint32_t* out_size);

std::int32_t cruspy_schema_decode_document_field(
    const std::uint8_t* data, std::uint32_t byte_size, const char* field_name,
    std::uint8_t* out_tag, std::int64_t* out_int, double* out_float, std::uint8_t* out_bool,
    char* out_string, std::uint32_t out_string_capacity, std::uint32_t* out_string_size);

}  // extern "C"
