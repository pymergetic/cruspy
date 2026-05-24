#include "_init.hpp"

int cruspy_substrate_handle_valid(const pymergetic::cruspy::substrate::MemoryHandle* handle) {
    if (handle == nullptr) {
        return 0;
    }
    return handle->abi_version == pymergetic::cruspy::substrate::kMemoryAbiVersion ? 1 : 0;
}
