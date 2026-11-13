"""TOML emitter adapted from gpui-toolkit/scripts/import_gpui_upstream.py.

Copyright 2025-2025 Pierre F. Aubert

Permission to use, copy, modify, and/or distribute this software for any purpose
with or without fee is hereby granted, provided that the above copyright notice
and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND
FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS
OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER
TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.
"""
import json
import re

class InlineTable(dict):
    """A dict rendered as a TOML inline table instead of a section."""


def _toml_value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, int):
        return str(v)
    if isinstance(v, list):
        return "[" + ", ".join(_toml_value(i) for i in v) + "]"
    if isinstance(v, InlineTable):
        inner = ", ".join(f"{_quote_segment(k)} = {_toml_value(x)}" for k, x in v.items())
        return "{ " + inner + " }" if inner else "{}"
    raise TypeError(f"unsupported TOML value: {v!r}")


def _quote_segment(seg: str) -> str:
    if re.fullmatch(r"[A-Za-z0-9_-]+", seg):
        return seg
    return json.dumps(seg)


def dumps_toml(doc: dict) -> str:
    lines: list[str] = []

    def emit(prefix: list[str], table: dict) -> None:
        scalars = {k: v for k, v in table.items()
                   if not isinstance(v, dict) or isinstance(v, InlineTable)}
        children = {k: v for k, v in table.items()
                    if isinstance(v, dict) and not isinstance(v, InlineTable)}
        # Header only for tables with own keys; pure-parent tables (e.g.
        # "target") must not emit intermediate headers.
        if prefix and scalars:
            lines.append("[" + ".".join(_quote_segment(s) for s in prefix) + "]")
        for k, v in scalars.items():
            lines.append(f"{_quote_segment(k)} = {_toml_value(v)}")
        if scalars:
            lines.append("")
        for name, child in children.items():
            emit(prefix + [name], child)

    emit([], doc)
    return "\n".join(lines).rstrip() + "\n"


