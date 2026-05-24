#pragma once

#include <exception>
#include <string>
#include <utility>

#include "errors/mod.hpp"

namespace pymergetic::cruspy::bridge {

inline std::string format_error(const CruspyError& err) {
  return std::string(error_code(err)) + ":" + err.what();
}

template <typename Fn>
auto safe_call(Fn&& fn) -> decltype(fn()) {
  using Result = decltype(fn());
  try {
    if constexpr (std::is_void_v<Result>) {
      fn();
      return;
    } else {
      return fn();
    }
  } catch (const ValidationError& err) {
    throw BridgeError(format_error(err));
  } catch (const AllocationError& err) {
    throw BridgeError(format_error(err));
  } catch (const ShmError& err) {
    throw BridgeError(format_error(err));
  } catch (const SchemaConflictError& err) {
    throw BridgeError(format_error(err));
  } catch (const TimeoutError& err) {
    throw BridgeError(format_error(err));
  } catch (const CruspyError& err) {
    throw BridgeError(format_error(err));
  } catch (const std::exception& err) {
    throw BridgeError(std::string("cruspy.bridge:") + err.what());
  }
}

}  // namespace pymergetic::cruspy::bridge
