"""Document model (EP-0021)."""

from __future__ import annotations

from pymergetic.cruspy.runtime import CRUSPY_REGISTER_METHOD

from .__init___gen import Document

__all__ = ["Document"]


def _score_text(self, text: str, model_id: str = "default") -> float:
    base = min(1.0, len(text) / 100.0)
    if model_id == "default":
        return base
    return min(1.0, base * 0.9)


CRUSPY_REGISTER_METHOD(Document, "score_text", _score_text)
