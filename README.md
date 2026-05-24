# cruspy

**cruspy — polyglot shared-memory runtime**

C++ data layer, Rust mediation bridge, Python/Pydantic on top. Define models once in
``mod.hpp``; codegen fills Rust and Python (EP-0010–EP-0017 in ``packages/ep/``).

| Surface | Value |
|---------|-------|
| **Repository** | [github.com/pymergetic/cruspy](https://github.com/pymergetic/cruspy) |
| **PyPI** | ``pymergetic-cruspy`` (when published) |
| **Import** | ``pymergetic.cruspy`` |
| **Monorepo path** | ``packages/cruspy/`` |

## Status

**Phase 1 (monolith, heap default)** — working ``Document`` model end-to-end:

- C++ validation in ``src/pymergetic/cruspy/models/document/mod.cpp``
- Rust ``cxx`` bridge + PyO3 root module
- Pydantic ``Document`` via ``create_model()``
- ``SCHEMA_HASH`` / ``TYPE_FQN`` metadata (EP-0012 prep)
- Error hierarchy stub in ``src/pymergetic/cruspy/errors/mod.hpp``

Phase 2 (plugins, SHM-default runtime) and phase 3 (async) are not started.

## Quick start (dev)

```bash
cd packages/cruspy
uv sync
uv run --with maturin maturin develop --release
uv run --with pytest pytest -v
python -c "from pymergetic.cruspy.models.document import Document; print(Document(id=1, text='hi', score=0.5))"
```

## Source tree (phase 1)

```
src/pymergetic/cruspy/
├── errors/mod.hpp          # EP-0015 error hierarchy (C++ SOT)
├── models/document/
│   ├── mod.hpp             # cxx bridge declarations
│   ├── mod.cpp             # C++ validation
│   └── types.hpp           # CRUSPY_MODEL(Document) heap definition
├── model.hpp               # CRUSPY_MODEL macro
└── runtime/mod.hpp         # C++ runtime version

src/models/document.rs      # cxx bridge + Pydantic factory (hand-written; EP-0012 codegen later)
src/lib.rs                  # PyO3 entry: pymergetic.cruspy
```

## Specification

Design is specified in the os-sdk EP series:

- EP-0010 — unified path convention
- EP-0011 — three-layer runtime
- EP-0012 — Rust codegen
- EP-0013 — plugin architecture (phase 2)
- EP-0014 — ``CRUSPY_MODEL`` type system
- EP-0015 — error propagation
- EP-0016 — async bridge (phase 3)
- EP-0017 — allocator registry

## License

Apache-2.0
