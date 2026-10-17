#!/usr/bin/env python3
"""Compare actual open-state Gallery PNG captures from baseline and current probes."""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageStat


parser = argparse.ArgumentParser()
parser.add_argument("baseline", type=Path)
parser.add_argument("current", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)

baseline_files = {p.name: p for p in args.baseline.glob("*.png")}
current_files = {p.name: p for p in args.current.glob("*.png")}
if baseline_files.keys() != current_files.keys():
    missing_baseline = sorted(current_files.keys() - baseline_files.keys())
    missing_current = sorted(baseline_files.keys() - current_files.keys())
    raise SystemExit(
        f"capture sets differ; missing baseline={missing_baseline}, missing current={missing_current}"
    )

results = []
diff_dir = args.output / "diff"
diff_dir.mkdir(exist_ok=True)
for name in sorted(baseline_files):
    before = Image.open(baseline_files[name]).convert("RGBA")
    after = Image.open(current_files[name]).convert("RGBA")
    if before.size != after.size:
        raise SystemExit(f"dimension mismatch for {name}: {before.size} != {after.size}")
    diff = ImageChops.difference(before, after)
    channels = diff.getchannel("R"), diff.getchannel("G"), diff.getchannel("B"), diff.getchannel("A")
    maximum_difference = ImageChops.lighter(
        ImageChops.lighter(channels[0], channels[1]),
        ImageChops.lighter(channels[2], channels[3]),
    )
    changed = maximum_difference.point(lambda value: 255 if value else 0).histogram()[255]
    maximum = max(channel.getextrema()[1] for channel in channels)
    mean_abs = sum(ImageStat.Stat(channel).mean[0] for channel in channels) / 4
    bbox = diff.getbbox(alpha_only=False)
    diff.save(diff_dir / name)
    results.append({
        "id": name.removesuffix(".png"),
        "size_px": before.size,
        "changed_pixels": changed,
        "changed_fraction": changed / (before.width * before.height),
        "max_channel_delta": maximum,
        "mean_abs_channel_delta": mean_abs,
        "diff_bbox": bbox,
        "pixel_identical": changed == 0,
    })

summary = {
    "capture_count": len(results),
    "pixel_identical_count": sum(item["pixel_identical"] for item in results),
    "cases": results,
}
(args.output / "pixel-comparison.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps({k: v for k, v in summary.items() if k != "cases"}, sort_keys=True))
