// Hello 3×3 dispatch matrix — C++ caller row (EP-0021).
//
//   cpp_calls_cpp     → Hello::hello_cpp()    → C++ impl
//   cpp_calls_rust    → Hello::hello_rust()   → Rust impl
//   cpp_calls_python  → Hello::hello_python() → Python impl

#include "../../models/hello/__init__.hpp"

#include <string>
#include <vector>

namespace {

using pymergetic::cruspy::models::hello::Hello;

constexpr const char* kMessage = "cruspy";

std::vector<std::uint8_t> expected(const char* lang) {
    const std::string text = std::string("Hello from ") + lang + " — " + kMessage;
    return std::vector<std::uint8_t>(text.begin(), text.end());
}

int check_bytes(const std::vector<std::uint8_t>& actual, const char* lang) {
    return actual == expected(lang) ? 1 : 0;
}

int cpp_calls_impl(const char* lang, std::vector<std::uint8_t> (Hello::*method)() const) {
    Hello hello("heap_default", kMessage);
    return check_bytes((hello.*method)(), lang);
}

}  // namespace

extern "C" {

int cruspy_test_cpp_calls_cpp() {
    return cpp_calls_impl("C++", &Hello::hello_cpp);
}

int cruspy_test_cpp_calls_rust() {
    return cpp_calls_impl("Rust", &Hello::hello_rust);
}

int cruspy_test_cpp_calls_python() {
    return cpp_calls_impl("Python", &Hello::hello_python);
}

}  // extern "C"
