"""TOML fixtures for subprocess tests; production uses Summer's own loader."""
import copy
import json
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def merge(base, overlay):
    result = copy.deepcopy(base)
    for key, value in overlay.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)
    return result


def settings():
    # Keep real test credentials in the ignored local TOML, not Python or env.
    example = ROOT / "config/app-dev.example.toml"
    local = ROOT / "config/app-dev.toml"
    with example.open("rb") as source:
        result = tomllib.load(source)
    if local.exists():
        with local.open("rb") as source:
            result = merge(result, tomllib.load(source))
    return result


def write(path, value):
    """Write typed fixture values without shell substitution or quoting secrets."""
    lines = []

    def table(prefix, values):
        if prefix:
            lines.append("[" + ".".join(prefix) + "]")
        for key, item in values.items():
            if item is not None and not isinstance(item, dict):
                lines.append(f"{key} = {json.dumps(item, ensure_ascii=False)}")
        lines.append("")
        for key, item in values.items():
            if isinstance(item, dict):
                table(prefix + [key], item)

    table([], value)
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")
