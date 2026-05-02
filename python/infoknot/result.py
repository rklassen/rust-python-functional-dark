"""Result-style return type and structured semantic error.

This module is the universal return-shape carrier for any operation in
``snap-py`` that can fail with a domain-meaningful reason.  The error is
``SemanticErr`` — a frozen dataclass that ALSO inherits from ``Exception``
so emergency ``raise`` at API boundaries remains possible without losing
the structured payload.

PEP 695 syntax:

    type Result[T] = Ok[T] | Err

is the public alias used throughout the package; pattern-match on ``Ok``
and ``Err`` for narrowing.

Invariants
----------

* ``SemanticErr.consider`` MUST be a non-empty tuple.  An empty consider
  list defeats the whole point of the error shape (no remediation hint),
  so the constructor rejects it with ``ValueError``.
* ``Ok[T]`` is generic via PEP 695.  ``Err`` is not generic — the error
  shape carries no type variable.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class SemanticErr(Exception):
    """A structured, presentation-aware error carrier.

    Subclasses ``Exception`` so it can be raised at boundaries when the
    surrounding callable does not return ``Result``.  When a callable
    DOES return ``Result``, prefer ``Err(SemanticErr(...))``.
    """

    found: str
    expected: str | None
    consider: tuple[str, ...]

    def __post_init__(self) -> None:
        if not self.consider:
            raise ValueError(
                "SemanticErr.consider must be non-empty",
            )
        # Initialise the Exception args so str(self) == self.pretty().
        # Frozen dataclasses block direct __init__ calls so we go through
        # Exception.args via the underlying object machinery.
        Exception.__init__(self, self.pretty())

    def pretty(self) -> str:
        """Return the multi-line human-readable form of this error."""
        lines = [f"found: {self.found}"]
        if self.expected is not None:
            lines.append(f"expected: {self.expected}")
        lines.append("consider:")
        lines.extend(f"  - {c}" for c in self.consider)
        return "\n".join(lines)


@dataclass(frozen=True, slots=True)
class Ok[T]:
    """Successful result carrying a value of type ``T``."""

    value: T


@dataclass(frozen=True, slots=True)
class Err:
    """Failed result carrying a ``SemanticErr``."""

    err: SemanticErr


type Result[T] = Ok[T] | Err
