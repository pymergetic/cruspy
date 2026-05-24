#include "__init__.hpp"

namespace pymergetic::cruspy::module {

ModuleNode::ModuleNode(std::string name, ModuleNode* parent) : name_(std::move(name)), parent_(parent) {}

const std::string& ModuleNode::name() const { return name_; }

std::string ModuleNode::full_name() const {
    if (parent_ == nullptr || parent_->name_.empty()) {
        return name_;
    }
    return parent_->full_name() + "." + name_;
}

ModuleNode* ModuleNode::parent() const { return parent_; }

bool ModuleNode::initialized() const { return initialized_; }

ModuleNode& ModuleNode::ensure_child(std::string name) {
    std::unique_lock lock(mutex_);
    auto it = children_.find(name);
    if (it != children_.end()) {
        return *it->second;
    }
    auto child = std::unique_ptr<ModuleNode>(new ModuleNode(std::move(name), this));
    auto& ref = *child;
    children_.emplace(ref.name_, std::move(child));
    return ref;
}

ModuleNode* ModuleNode::find_child(std::string_view name) const {
    std::shared_lock lock(mutex_);
    const auto it = children_.find(std::string(name));
    return it == children_.end() ? nullptr : it->second.get();
}

std::vector<const ModuleNode*> ModuleNode::children() const {
    std::shared_lock lock(mutex_);
    std::vector<const ModuleNode*> out;
    out.reserve(children_.size());
    for (const auto& [_, child] : children_) {
        out.push_back(child.get());
    }
    return out;
}

ModuleNode& ModuleNode::root() {
    static ModuleNode root_node{"", nullptr};
    return root_node;
}

ModuleNode* ModuleNode::find(std::string_view full_name) {
    ModuleNode* node = &root();
    if (full_name.empty()) {
        return node;
    }

    std::size_t start = 0;
    while (start < full_name.size()) {
        std::size_t dot = full_name.find('.', start);
        if (dot == std::string_view::npos) {
            dot = full_name.size();
        }
        const auto segment = full_name.substr(start, dot - start);
        if (!segment.empty()) {
            node = node->find_child(segment);
            if (node == nullptr) {
                return nullptr;
            }
        }
        start = dot + 1;
    }
    return node;
}

ModuleNode& ModuleNode::ensure(std::string_view full_name) {
    ModuleNode* node = &root();
    if (full_name.empty()) {
        return *node;
    }

    std::size_t start = 0;
    while (start < full_name.size()) {
        std::size_t dot = full_name.find('.', start);
        if (dot == std::string_view::npos) {
            dot = full_name.size();
        }
        const auto segment = full_name.substr(start, dot - start);
        if (!segment.empty()) {
            node = &node->ensure_child(std::string(segment));
        }
        start = dot + 1;
    }
    return *node;
}

ModuleNode& ModuleNode::attach(std::string_view full_name, InitCallback init, ShutdownCallback shutdown) {
    ModuleNode& node = ensure(full_name);
    {
        std::unique_lock lock(node.mutex_);
        node.callbacks_.push_back(CallbackEntry{init, shutdown, false});
        node.initialized_ = false;
    }
    return node;
}

void ModuleNode::apply_postorder() {
    std::vector<ModuleNode*> child_nodes;
    {
        std::shared_lock lock(mutex_);
        child_nodes.reserve(children_.size());
        for (auto& [_, child] : children_) {
            child_nodes.push_back(child.get());
        }
    }
    for (auto* child : child_nodes) {
        child->apply_postorder();
    }

    std::vector<InitCallback> pending;
    {
        std::unique_lock lock(mutex_);
        for (auto& entry : callbacks_) {
            if (!entry.applied && entry.init != nullptr) {
                pending.push_back(entry.init);
                entry.applied = true;
            }
        }
        if (!pending.empty()) {
            initialized_ = true;
        }
    }
    for (auto init : pending) {
        init();
    }
}

void ModuleNode::shutdown_postorder() {
    std::vector<ModuleNode*> child_nodes;
    {
        std::shared_lock lock(mutex_);
        child_nodes.reserve(children_.size());
        for (auto& [_, child] : children_) {
            child_nodes.push_back(child.get());
        }
    }
    for (auto* child : child_nodes) {
        child->shutdown_postorder();
    }

    std::vector<ShutdownCallback> pending;
    {
        std::unique_lock lock(mutex_);
        for (auto it = callbacks_.rbegin(); it != callbacks_.rend(); ++it) {
            if (it->applied && it->shutdown != nullptr) {
                pending.push_back(it->shutdown);
                it->applied = false;
            }
        }
        if (!pending.empty()) {
            initialized_ = false;
        }
    }
    for (auto shutdown : pending) {
        shutdown();
    }
}

void ModuleNode::apply_all() { root().apply_postorder(); }

void ModuleNode::apply_subtree(std::string_view full_name) {
    ensure(full_name).apply_postorder();
}

void ModuleNode::shutdown_all() { root().shutdown_postorder(); }

ModuleRegistrar::ModuleRegistrar(const char* full_name, InitCallback init, ShutdownCallback shutdown) {
    if (full_name != nullptr && init != nullptr) {
        ModuleNode::attach(full_name, init, shutdown);
    }
}

}  // namespace pymergetic::cruspy::module

extern "C" {

void cruspy_module_ensure(const char* full_name) {
    if (full_name == nullptr) {
        return;
    }
    pymergetic::cruspy::module::ModuleNode::ensure(full_name);
}

void cruspy_module_apply_all(void) { pymergetic::cruspy::module::ModuleNode::apply_all(); }

void cruspy_module_apply_subtree(const char* full_name) {
    if (full_name == nullptr) {
        return;
    }
    pymergetic::cruspy::module::ModuleNode::apply_subtree(full_name);
}

void cruspy_module_shutdown_all(void) { pymergetic::cruspy::module::ModuleNode::shutdown_all(); }

void cruspy_so_entry(void) { pymergetic::cruspy::module::ModuleNode::apply_all(); }

}  // extern "C"
