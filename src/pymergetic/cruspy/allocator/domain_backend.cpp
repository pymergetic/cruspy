#include "allocator/domain_backend.hpp"

#include <atomic>

namespace pymergetic::cruspy::allocator {

namespace {

std::atomic<std::uint64_t> g_domain_counter{1};

}  // namespace

DomainId next_domain_id() {
  const auto value = g_domain_counter.fetch_add(1, std::memory_order_relaxed);
  return DomainId{.high = 0, .low = value};
}

}  // namespace pymergetic::cruspy::allocator
