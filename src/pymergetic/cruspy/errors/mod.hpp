#pragma once

#include <stdexcept>
#include <string>
#include <string_view>

namespace pymergetic::cruspy {

class CruspyError : public std::runtime_error {
 public:
  explicit CruspyError(std::string message)
      : std::runtime_error(std::move(message)) {}
};

class ValidationError : public CruspyError {
 public:
  explicit ValidationError(std::string message)
      : CruspyError(std::move(message)) {}
};

class AllocationError : public CruspyError {
 public:
  explicit AllocationError(std::string message)
      : CruspyError(std::move(message)) {}
};

class BridgeError : public CruspyError {
 public:
  explicit BridgeError(std::string message)
      : CruspyError(std::move(message)) {}
};

class ShmError : public CruspyError {
 public:
  explicit ShmError(std::string message)
      : CruspyError(std::move(message)) {}
};

class SchemaConflictError : public CruspyError {
 public:
  explicit SchemaConflictError(std::string message)
      : CruspyError(std::move(message)) {}
};

class TimeoutError : public CruspyError {
 public:
  explicit TimeoutError(std::string message)
      : CruspyError(std::move(message)) {}
};

inline constexpr std::string_view error_code(const CruspyError&) {
  return "cruspy.error";
}

inline constexpr std::string_view error_code(const ValidationError&) {
  return "cruspy.validation";
}

inline constexpr std::string_view error_code(const AllocationError&) {
  return "cruspy.allocation";
}

inline constexpr std::string_view error_code(const BridgeError&) {
  return "cruspy.bridge";
}

inline constexpr std::string_view error_code(const ShmError&) { return "cruspy.shm"; }

inline constexpr std::string_view error_code(const SchemaConflictError&) {
  return "cruspy.schema_conflict";
}

inline constexpr std::string_view error_code(const TimeoutError&) {
  return "cruspy.timeout";
}

}  // namespace pymergetic::cruspy
