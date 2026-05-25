#include "__init__.hpp"

#include "../../registry/__init__.hpp"

#include <cstring>
#include <string>

namespace pymergetic::cruspy::models::hello {

int hello_cpp(const substrate::MemoryHandle* handle, std::uint8_t* out, std::size_t capacity) {
    if (handle == nullptr) {
        return -1;
    }
    char buf[256];
    const int n = registry::field_get_string(*handle, "message", buf, sizeof(buf));
    const std::string result =
        std::string("Hello from C++ — ") + (n >= 0 ? std::string(buf, static_cast<std::size_t>(n)) : "");
    if (capacity == 0) {
        return static_cast<int>(result.size());
    }
    if (out == nullptr || capacity < result.size()) {
        return -1;
    }
    std::memcpy(out, result.data(), result.size());
    return static_cast<int>(result.size());
}

}  // namespace pymergetic::cruspy::models::hello

CRUSPY_REGISTER_METHOD(pymergetic::cruspy::models::hello::HelloLayout, hello_cpp,
                       pymergetic::cruspy::models::hello::hello_cpp)
