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

## Architecture

| Concern | Owner |
|---------|--------|
| Model IDL (``.openapi.yaml``) | **cruspy-gen** (``tools/cruspy-gen/``) |
| Generated field layouts + typed wrappers | ``{stem}_gen.{hpp,cpp,rs,py}`` |
| Generated C++ public include | ``__init__.hpp`` (forwards to ``{stem}_gen.hpp``) |
| Hand-written method bodies | ``__init__.{hpp,cpp,rs,py}`` |
| Registry, allocator, dispatch | C++ kernel + PyO3 ``runtime`` |

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
