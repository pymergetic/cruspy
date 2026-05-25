# cruspy

**cruspy — polyglot shared-memory runtime**

C++ data layer, Rust mediation bridge, Python on top. Models are defined in
**OpenAPI 3.1** YAML (EP-0021); **cruspy-gen** emits C++, Rust, and Python
from a single source of truth.

| Surface | Value |
|---------|-------|
| **Repository** | [github.com/pymergetic/cruspy](https://github.com/pymergetic/cruspy) |
| **PyPI** | ``pymergetic-cruspy`` (when published) |
| **Import** | ``pymergetic.cruspy`` |
| **Monorepo path** | ``packages/cruspy/`` |

## The Point

**Hello** is the proof: **three methods, each defined in a different language**, all
on the **same shared-memory object**. OpenAPI declares them; you hand-write **one
function in its native lang**; the registry lets **any caller** invoke **any method**.

| Method | **Defined in** | Hand-written file |
|--------|----------------|-------------------|
| `hello_cpp` | **C++** | `models/hello/__init__.cpp` |
| `hello_rust` | **Rust** | `models/hello/__init__.rs` |
| `hello_python` | **Python** | `models/hello/__init__.py` |

Same object, same `message` field — three native implementations, zero rewrites in
the other languages.

### Define — one func per language

**C++** defines `hello_cpp`:

```cpp
// models/hello/__init__.cpp
int hello_cpp(const MemoryHandle* handle, uint8_t* out, size_t capacity) {
    // read message field, write "Hello from C++ — …" bytes
}
CRUSPY_REGISTER_METHOD(Hello, hello_cpp, hello_cpp)
```

**Rust** defines `hello_rust`:

```rust
// models/hello/__init__.rs
#[no_mangle]
pub unsafe extern "C" fn hello_rust(handle: *const MemoryHandle, out: *mut u8, cap: usize) -> i32 {
    // read message field, write "Hello from Rust — …" bytes
}
CRUSPY_REGISTER_METHOD!(Hello, hello_rust, hello_rust);
```

**Python** defines `hello_python`:

```python
# models/hello/__init__.py
def hello_python(self) -> bytes:
    return f"Hello from Python — {self.field_string('message')}".encode()

CRUSPY_REGISTER_METHOD(Hello, hello_python)
```

All three use the **same model type** — ``Hello`` — and the same macro name —
``CRUSPY_REGISTER_METHOD``. Args are always ``(Model, method_name, function)``;
Python omits the method name string when it matches the function name.

### Call — usage from every language

Generated wrappers are symmetric. From **any** language you call all three methods;
dispatch crosses the vtable to the native impl.

**Python**

```bash
uv run python scripts/demo_crosslang.py
```

```python
from pymergetic.cruspy.models.hello import Hello

h = Hello(message="cruspy")
print(h.hello_cpp().decode())     # → runs C++ impl
print(h.hello_rust().decode())    # → runs Rust impl
print(h.hello_python().decode())  # → runs Python impl
```

**C++**

```cpp
#include "models/hello/__init__.hpp"

using pymergetic::cruspy::models::hello::Hello;

Hello h("heap_default", "cruspy");
auto from_cpp    = h.hello_cpp();     // → runs C++ impl
auto from_rust   = h.hello_rust();    // → runs Rust impl
auto from_python = h.hello_python();  // → runs Python impl
```

**Rust**

```rust
use crate::cruspy_root::models::hello::{Hello, HelloInit};

let h = Hello::new("heap_default", HelloInit { message: "cruspy".into() })?;
let from_cpp    = h.hello_cpp()?;     // → runs C++ impl
let from_rust   = h.hello_rust()?;    // → runs Rust impl
let from_python = h.hello_python()?;  // → runs Python impl
```

### Proof: 3×3 dispatch matrix

Defining in three languages is only half the story — **callers** also work from all
three. ``tests/hello/test_dispatch_matrix.py`` runs every **caller × impl** cell;
test names are ``{caller}_calls_{impl}``:

```
caller \ impl   cpp              rust             python
python          python_calls_cpp python_calls_rust python_calls_python
cpp             cpp_calls_cpp    cpp_calls_rust   cpp_calls_python
rust            rust_calls_cpp   rust_calls_rust  rust_calls_python
```

```bash
uv run pytest tests/hello/ -vv    # 9 matrix cells + 1 field check = 10 passed
```

C++ / Rust caller rows: ``testing/hello/`` native harness. Python row: generated
``Hello`` API. Same registry vtable everywhere.

## Architecture

| Concern | Owner |
|---------|--------|
| Model IDL (``.openapi.yaml``) | **cruspy-gen** (``tools/cruspy-gen/``) |
| Generated field layouts + typed wrappers | ``{stem}_gen.{hpp,cpp,rs,py}`` |
| Generated C++ public include | ``__init__.hpp`` (forwards to ``{stem}_gen.hpp``) |
| Hand-written method bodies | ``__init__.{hpp,cpp,rs,py}`` |
| Registry, allocator, dispatch | C++ kernel + PyO3 ``runtime`` |

### Shared memory (tri-language, one region)

C++ defines the **object layout** and owns the **allocator**. Rust and Python never
allocate a parallel copy of model fields — they hold a ``MemoryHandle`` (domain +
offset) into the same bytes.

In the hybrid extension (maturin build), all three languages link one kernel:

```
  Python Hello._handle  ──┐
  Rust   Hello.handle   ──┼──► MemoryHandle ──► C++ domain bytes (ObjectHeader + fields)
  C++    Hello.handle_  ──┘
```

- **Same object** — ``hello_cpp`` / ``hello_rust`` / ``hello_python`` all receive
  that handle; ``message`` is read from one slot in the blob.
- **In-place fields** — numeric/bool field get/set goes through the registry into
  the allocation (see ``test_shm_domain_zero_copy_reattach`` for handle clone +
  ``shm_default``).
- **Not magic** — method return buffers (``call_bytes``, ``bytes``, ``Vec<u8>``) and
  string field APIs copy out; cross-process sharing needs an explicit SHM domain, not
  the default ``heap_default`` heap.

So: **one memory region, three language surfaces** — transient handles and dispatch,
not three separate model heaps.

**cruspy does not link easybind at runtime.** Codegen is driven by OpenAPI YAML
and runs automatically from ``build.rs`` (via ``cruspy-build``) and CMake
(``cmake/CruspyGen.cmake``).

## Quick start (dev)

```bash
cd packages/cruspy
uv sync
uv run --with maturin maturin develop --release
uv run --with pytest pytest -v
python -c "from pymergetic.cruspy.models.document import Document; print(Document(id=1, score=0.5))"
```

### Codegen workflow (EP-0021)

1. Edit ``models/<model>/{stem}.openapi.yaml`` (e.g. ``__init__.openapi.yaml``)
2. Rebuild — ``build.rs`` / CMake runs ``cruspy-gen`` → ``{stem}_gen.*``
3. Add hand method implementations in ``__init__.{hpp,cpp,rs,py}`` when needed

```bash
# Manual regen (optional)
uv run --with pyyaml --with jinja2 python tools/cruspy-gen/cruspy_gen.py \
  --root src/pymergetic/cruspy --glob "models/**/*.openapi.yaml"

# CI staleness check
bash scripts/check_codegen.sh
```

## Source tree

```
src/pymergetic/cruspy/
├── models/document/
│   ├── __init__.openapi.yaml   # package SOT → __init___gen.*
│   ├── __init___gen.*          # AUTO-GENERATED (never edit)
│   ├── __init__.hpp            # hand C++ public include (or autogen forward if absent)
│   └── __init__.{cpp,rs,py}    # hand method impls
│   └── metadata/__init___gen.* # nested schema outputs
│   # file-module variant: xyz.openapi.yaml → xyz_gen.*
├── models/hello/               # cross-language demo (EP-0021)
│   ├── hello.openapi.yaml
│   ├── hello_gen.*
│   └── __init__.{hpp,cpp,rs,py}
├── testing/hello/              # native 3×3 dispatch harness for pytest
├── registry/                   # TypeRegistry + call_* dispatch (__init__.*)
├── runtime/                    # PyO3 MemoryHandle + create/describe (__init__.rs)
├── substrate/                  # MemoryHandle layout (__init__.hpp)
tools/cruspy-gen/               # OpenAPI → tri-language generator
cruspy-build/                   # build.rs helper crate
cmake/CruspyGen.cmake           # IDE/clangd codegen hook
```

## Specification

Design is specified in the os-sdk EP series (EP-0010–EP-0021): memory substrate,
schema registry, OpenAPI IDL, and cross-language method dispatch.

## License

Apache-2.0 — see [LICENSE](LICENSE).
