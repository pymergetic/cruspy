#pragma once

// EP-0015 — error types (single source of truth). Mirrored to Rust/Python by codegen.

namespace pymergetic::cruspy {

struct CruspyError {
  const char* what() const noexcept;
};

struct ValidationError : CruspyError {};
struct AllocationError : CruspyError {};
struct BridgeError : CruspyError {};
struct ShmError : CruspyError {};
struct SchemaConflictError : CruspyError {};
struct TimeoutError : CruspyError {};

}  // namespace pymergetic::cruspy
