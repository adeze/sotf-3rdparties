#!/usr/bin/env python3
"""Render current GPUI identity from FORKS.json and package manifests."""
import argparse
import json
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
START = '<!-- current-gpui-provenance:start -->'
END = '<!-- current-gpui-provenance:end -->'


def current_blocks(root: Path) -> dict[Path, str]:
    inventory = json.loads((root / 'FORKS.json').read_text())
    fork = next(item for item in inventory['forks'] if item['directory'] == 'gpui')
    crates = root / 'gpui/crates'
    rows = []
    for member in fork['members'] + fork['support_crates']:
        path = 'refineable/derive_refineable' if member == 'derive_refineable' else member
        package = tomllib.loads((crates / path / 'Cargo.toml').read_text())['package']
        rows.append(f"| `{path}` | `{package['name']}` | `{package['version']}` |")
    overview = '\n'.join([
        START,
        f"Current upstream ref: `{fork['upstream_revision']}`. Support source revision: `{fork['support_upstream_revision']}`.",
        '', '| Crate | Local package | Version |', '| --- | --- | --- |', *rows, END,
    ])
    package = tomllib.loads((crates / 'gpui_windows/Cargo.toml').read_text())['package']
    windows = '\n'.join([
        START,
        f"Current upstream ref: `{fork['upstream_revision']}`.",
        f"Local package: `{package['name']}` `{package['version']}` (`gpui_windows` library).",
        END,
    ])
    return {root / 'gpui/README.md': overview,
            crates / 'gpui_windows/VENDORING.md': windows}


def update(root: Path, write: bool = False) -> list[str]:
    errors = []
    for path, expected in current_blocks(root).items():
        content = path.read_text()
        if content.count(START) != 1 or content.count(END) != 1:
            errors.append(f'{path}: missing or duplicate provenance markers')
            continue
        before, remainder = content.split(START, 1)
        _, after = remainder.split(END, 1)
        rendered = before + expected + after
        if rendered != content:
            if write:
                path.write_text(rendered)
            else:
                errors.append(f'{path}: current GPUI provenance differs from inventory or manifests; run scripts/gpui_provenance.py --write')
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    errors = update(ROOT, args.write)
    if errors:
        raise SystemExit('\n'.join(errors))
    print('GPUI provenance is current.')


if __name__ == '__main__':
    main()
