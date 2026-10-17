#!/usr/bin/env python3
"""Create a reproducible target-only Gallery probe source clone.

The baseline clone restores the two files that differ from the frozen-source
manifest from the previous verified source archive. The current clone is made
from the live checkout only when requested, after its implementation is stable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ARTIFACT = ROOT / "target/tabs-menu-dialog-20261016"
FROZEN = ARTIFACT / "before-source.json"
SOURCE_ARCHIVE = ROOT / "design/reviews/component-layout-followup-20261016/changed-sources.tar.gz"
PATCHED_FILES = (
    "crates/tekes-ui-controls/src/lib.rs",
    "crates/tekes-ui-gallery/src/gallery/workbench.rs",
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def clone_workspace(name: str, baseline: bool) -> dict[str, str]:
    destination = ARTIFACT / "sources" / name
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    shutil.copy2(ROOT / "Cargo.toml", destination / "Cargo.toml")
    shutil.copy2(ROOT / "Cargo.lock", destination / "Cargo.lock")
    shutil.copytree(ROOT / "crates", destination / "crates", symlinks=False)
    for directory in ("design", "fixtures", "scripts"):
        (destination / directory).symlink_to(ROOT / directory, target_is_directory=True)

    before = json.loads(FROZEN.read_text())
    archive_sha: dict[str, str] = {}
    if baseline:
        with tarfile.open(SOURCE_ARCHIVE, "r:gz") as archive:
            members = set(archive.getnames())
            for relative in PATCHED_FILES:
                if relative not in members:
                    raise RuntimeError(f"frozen source unavailable in archive: {relative}")
                content = archive.extractfile(relative).read()
                restored = destination / relative
                restored.write_bytes(content)
                digest = hashlib.sha256(content).hexdigest()
                if digest != before.get(relative):
                    raise RuntimeError(f"frozen SHA mismatch for {relative}: {digest}")
                archive_sha[relative] = digest

    gallery = destination / "crates/tekes-ui-gallery"
    workbench = gallery / "src/gallery/workbench.rs"
    source = workbench.read_text()
    if "pub mod probe;" not in source:
        source = source.replace("mod extended;", "mod extended;\npub mod probe;", 1)
        workbench.write_text(source)
    (gallery / "src/gallery/workbench/probe.rs").write_bytes(
        (ARTIFACT / "probe_module.rs").read_bytes()
    )
    (gallery / "src/bin").mkdir(exist_ok=True)
    (gallery / "src/bin/offscreen_overlay_probe.rs").write_bytes(
        (ARTIFACT / "probe_main.rs").read_bytes()
    )
    with (gallery / "Cargo.toml").open("a") as manifest:
        manifest.write(
            '\n[[bin]]\nname = "offscreen-overlay-probe"\n'
            'path = "src/bin/offscreen_overlay_probe.rs"\n'
            '\n[features]\nbaseline = []\ncurrent = []\n'
        )

    receipt = {
        "source_kind": name,
        "workspace": str(destination),
        "baseline_restored": archive_sha,
        "workbench_sha_after_probe_wrapper": sha256(workbench),
        "controls_lib_sha": sha256(destination / "crates/tekes-ui-controls/src/lib.rs"),
        "composite_sha": sha256(destination / "crates/tekes-ui-controls/src/composite.rs"),
    }
    (ARTIFACT / f"{name}-source-receipt.json").write_text(
        json.dumps(receipt, indent=2) + "\n"
    )
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", choices=("baseline", "current", "both"))
    args = parser.parse_args()
    selected = ("baseline", "current") if args.source == "both" else (args.source,)
    for name in selected:
        receipt = clone_workspace(name, baseline=name == "baseline")
        print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()
