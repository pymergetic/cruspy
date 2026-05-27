// Native cross-language dispatch checks (EP-0021).
#include "../models/document/__init__.hpp"
#include "../registry/__init__.hpp"

#include <cmath>
#include <cstring>

namespace {

using pymergetic::cruspy::models::document::Document;
using pymergetic::cruspy::models::document::metadata::Metadata;

}  // namespace

extern "C" {

int cruspy_test_cpp_validate() {
    Document doc("heap_default", 50, 0.5, true, Metadata("heap_default"));
    return doc.validate() ? 1 : 0;
}

int cruspy_test_cpp_normalize() {
    Document doc("heap_default", -5, 1.5, true, Metadata("heap_default"));
    doc.normalize();
    return (doc.id() == 0 && std::abs(doc.score() - 1.0) < 1e-9) ? 1 : 0;
}

int cruspy_test_cpp_serialize_rust() {
    Document doc("heap_default", 7, 0.875, true, Metadata("heap_default", 3, 99));
    const auto blob = doc.serialize();
    if (blob.size() != 29 || blob[0] != 'C' || blob[1] != 'D') {
        return -1;
    }
    return static_cast<int>(blob.size());
}

int cruspy_test_cpp_from_json_rust() {
    const Document doc = Document::from_json(
        R"({"id":3,"score":0.25,"active":false,"meta":{"id":8,"created_at":1234}})",
        "heap_default");
    if (doc.id() != 3) {
        return -1;
    }
    if (std::abs(doc.score() - 0.25) >= 1e-9) {
        return -2;
    }
    if (doc.active()) {
        return -3;
    }
    if (doc.meta().id() != 8 || doc.meta().created_at() != 1234) {
        return -4;
    }
    return 0;
}

double cruspy_test_cpp_score_text_python() {
    Document doc("heap_default", 1, 0.0, true, Metadata("heap_default"));
    return doc.score_text("hello world", "default");
}

}  // extern "C"
