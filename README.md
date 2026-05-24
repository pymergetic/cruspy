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

Phase 1 (monolith, heap default): scaffold only. See EP-0011 implementation phases.

## Quick start (dev)

```bash
cd packages/cruspy
uv sync
uv run maturin develop
python -c "import pymergetic.cruspy; print(pymergetic.cruspy.__doc__)"
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
