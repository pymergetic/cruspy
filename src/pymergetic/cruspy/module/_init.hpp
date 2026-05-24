#pragma once

#include "detail.hpp"

#include <cstddef>
#include <map>
#include <memory>
#include <mutex>
#include <shared_mutex>
#include <string>
#include <string_view>
#include <vector>

namespace pymergetic::cruspy::module {

/// Canonical import root — harmonized across C++, Rust, and Python.
inline constexpr const char* kPackageRoot = "pymergetic.cruspy";

using InitCallback = void (*)();
using ShutdownCallback = void (*)();

/// Unified module path tree (Meyer singleton root). Empty placeholder nodes keep
/// the hierarchy complete before every translation unit / shared object loads.
class ModuleNode {
public:
    const std::string& name() const;
    std::string full_name() const;
    ModuleNode* parent() const;

    bool initialized() const;
    std::vector<const ModuleNode*> children() const;

    static ModuleNode& root();
    static ModuleNode* find(std::string_view full_name);
    static ModuleNode& ensure(std::string_view full_name);
    static ModuleNode& attach(std::string_view full_name, InitCallback init, ShutdownCallback shutdown = nullptr);

    static void apply_all();
    static void apply_subtree(std::string_view full_name);
    static void shutdown_all();

private:
    struct CallbackEntry {
        InitCallback init = nullptr;
        ShutdownCallback shutdown = nullptr;
        bool applied = false;
    };

    explicit ModuleNode(std::string name, ModuleNode* parent);

    void apply_postorder();
    void shutdown_postorder();
    ModuleNode& ensure_child(std::string name);
    ModuleNode* find_child(std::string_view name) const;

    std::string name_;
    ModuleNode* parent_;
    mutable std::shared_mutex mutex_;
    std::map<std::string, std::unique_ptr<ModuleNode>> children_;
    std::vector<CallbackEntry> callbacks_;
    bool initialized_{false};
};

/// Static registration object — constructing one attaches init/shutdown on a path.
class ModuleRegistrar {
public:
    ModuleRegistrar(const char* full_name, InitCallback init, ShutdownCallback shutdown = nullptr);
};

}  // namespace pymergetic::cruspy::module

#ifdef __cplusplus
extern "C" {
#endif

void cruspy_module_ensure(const char* full_name);
void cruspy_module_apply_all(void);
void cruspy_module_apply_subtree(const char* full_name);
void cruspy_module_shutdown_all(void);
/// Entry point for a late-loaded shared object after its static registrars run.
void cruspy_so_entry(void);

#ifdef __cplusplus
}
#endif

#define CRUSPY_MODULE_CONCAT_INNER(a, b) a##b
#define CRUSPY_MODULE_CONCAT(a, b) CRUSPY_MODULE_CONCAT_INNER(a, b)

/// Derive module path from a C++ namespace token (``a::b`` → ``a.b``).
#define CRUSPY_NS_MODULE(NS, init_fn)                                                      \
    namespace {                                                                            \
    inline constexpr auto _cruspy_ns_path_##__LINE__ =                                     \
        ::pymergetic::cruspy::module::detail::ns_to_path_array(#NS);                       \
    [[gnu::used]] ::pymergetic::cruspy::module::ModuleRegistrar                            \
        CRUSPY_MODULE_CONCAT(_cruspy_ns_mod_, __LINE__)(                                    \
            _cruspy_ns_path_##__LINE__.data(), init_fn);                                   \
    }

/// Register module init at static initialization time (Meyers object pattern).
#define CRUSPY_MODULE(path_literal, init_fn)                                               \
    namespace {                                                                            \
    [[gnu::used]] ::pymergetic::cruspy::module::ModuleRegistrar                            \
        CRUSPY_MODULE_CONCAT(_cruspy_module_reg_, __LINE__)(path_literal, init_fn);        \
    }

#define CRUSPY_MODULE_WITH_SHUTDOWN(path_literal, init_fn, shutdown_fn)                    \
    namespace {                                                                            \
    [[gnu::used]] ::pymergetic::cruspy::module::ModuleRegistrar                            \
        CRUSPY_MODULE_CONCAT(_cruspy_module_reg_, __LINE__)(path_literal, init_fn,         \
                                                            shutdown_fn);                  \
    }
