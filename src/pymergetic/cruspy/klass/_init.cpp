#include "_init.hpp"

#include <string_view>

namespace pymergetic::cruspy::klass {

MetaStore& MetaStore::global() {
    static MetaStore store;
    return store;
}

const TypeMeta* MetaStore::find(std::type_index id) const {
    std::lock_guard lock(mutex_);
    const auto it = by_type_.find(id);
    return it == by_type_.end() ? nullptr : &it->second;
}

const TypeMeta* MetaStore::find_by_fqn(std::string_view fqn) const {
    std::lock_guard lock(mutex_);
    const auto it = by_fqn_.find(std::string(fqn));
    if (it == by_fqn_.end()) {
        return nullptr;
    }
    const auto type_it = by_type_.find(it->second);
    return type_it == by_type_.end() ? nullptr : &type_it->second;
}

}  // namespace pymergetic::cruspy::klass
