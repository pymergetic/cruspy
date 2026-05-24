#pragma once

#include "../cobject/__init__.hpp"
#include "../field/__init__.hpp"

#include <cstdint>
#include <mutex>
#include <string>
#include <string_view>
#include <typeindex>
#include <unordered_map>
#include <vector>

namespace pymergetic::cruspy::klass {

/// Tag base for registered object klasses.
struct KlassBase : cobject::CObject {
    static constexpr uint32_t kind_klass = 1;
};

struct TypeMeta {
    std::string module_path;
    std::string type_name;
    std::string fqn;
    std::vector<field::FieldMeta> fields;

    [[nodiscard]] std::string make_fqn() const { return module_path + "." + type_name; }
};

/// Runtime store for ``TypeMeta`` keyed by C++ type (separate from instance layout).
class MetaStore {
public:
    static MetaStore& global();

    template <typename T>
    const TypeMeta* find() const {
        std::lock_guard lock(mutex_);
        const auto it = by_type_.find(std::type_index(typeid(T)));
        return it == by_type_.end() ? nullptr : &it->second;
    }

    const TypeMeta* find(std::type_index id) const;
    const TypeMeta* find_by_fqn(std::string_view fqn) const;

    template <typename T>
    void emplace(TypeMeta meta) {
        std::lock_guard lock(mutex_);
        const std::type_index id(typeid(T));
        const std::string fqn = meta.fqn;
        by_type_.insert_or_assign(id, std::move(meta));
        by_fqn_.insert_or_assign(fqn, id);
    }

private:
    MetaStore() = default;

    mutable std::mutex mutex_;
    std::unordered_map<std::type_index, TypeMeta> by_type_;
    std::unordered_map<std::string, std::type_index> by_fqn_;
};

}  // namespace pymergetic::cruspy::klass
