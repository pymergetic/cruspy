#pragma once

#include <cstdint>
#include <cstring>
#include <string_view>

namespace pymergetic::cruspy::substrate {

inline constexpr uint32_t kMemoryAbiVersion = 1;
inline constexpr uint32_t kHandleFlagNone = 0x00;
inline constexpr uint32_t kHandleFlagTyped = 0x01;
inline constexpr uint32_t kHandleFlagStaleCheck = 0x02;
inline constexpr uint32_t kHandleFlagEmbedded = 0x08;

struct DomainId {
    uint64_t high{};
    uint64_t low{};
    auto operator<=>(const DomainId&) const = default;
};

struct MemoryHandle {
    uint32_t abi_version{kMemoryAbiVersion};
    uint32_t flags{kHandleFlagTyped | kHandleFlagStaleCheck};
    DomainId domain_id{};
    uint64_t offset{};
    uint64_t byte_size{};
    uint64_t schema_hash{};
    uint64_t generation{};
    uint64_t embedded_offset{};
    char type_fqn[24]{};
};

inline void handle_zero(MemoryHandle* handle) {
    if (handle == nullptr) {
        return;
    }
    *handle = MemoryHandle{};
    handle->abi_version = kMemoryAbiVersion;
    handle->flags = kHandleFlagTyped | kHandleFlagStaleCheck;
}

inline void handle_set_fqn(MemoryHandle* handle, std::string_view fqn) {
    if (handle == nullptr) {
        return;
    }
    std::memset(handle->type_fqn, 0, sizeof(handle->type_fqn));
    const auto n = fqn.size() < sizeof(handle->type_fqn) ? fqn.size() : sizeof(handle->type_fqn) - 1;
    std::memcpy(handle->type_fqn, fqn.data(), n);
}

struct ObjectHeader {
    uint64_t schema_hash{};
    uint32_t type_version{};
    uint32_t header_size{sizeof(ObjectHeader)};
    char type_fqn[64]{};
};

inline void header_init(ObjectHeader* header, uint64_t schema_hash, uint32_t version, std::string_view fqn) {
    if (header == nullptr) {
        return;
    }
    header->schema_hash = schema_hash;
    header->type_version = version;
    header->header_size = static_cast<uint32_t>(sizeof(ObjectHeader));
    std::memset(header->type_fqn, 0, sizeof(header->type_fqn));
    const auto n = fqn.size() < sizeof(header->type_fqn) ? fqn.size() : sizeof(header->type_fqn) - 1;
    std::memcpy(header->type_fqn, fqn.data(), n);
}

}  // namespace pymergetic::cruspy::substrate

#ifdef __cplusplus
extern "C" {
#endif

int cruspy_substrate_handle_valid(const pymergetic::cruspy::substrate::MemoryHandle* handle);

#ifdef __cplusplus
}
#endif
