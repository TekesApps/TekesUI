#!/usr/bin/env python3
"""校验 fixtures/text 下的夹具是否满足 README 与 manifest 中约定的规则。

只用标准库。用法：python3 check.py（任意目录均可）。
"""
import json
import re
import sys
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
errors = []


def err(msg):
    errors.append(msg)


def read(name):
    p = HERE / name
    raw = p.read_bytes()
    if raw.startswith(b"\xef\xbb\xbf"):
        err(f"{name}: 含 BOM")
    if b"\r" in raw:
        err(f"{name}: 含 CR（需要 LF 换行）")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as e:
        err(f"{name}: 不是合法 UTF-8: {e}")
        return []
    if text and not text.endswith("\n"):
        err(f"{name}: 末尾缺少换行")
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    for i, ln in enumerate(lines, 1):
        if ln != ln.rstrip():
            err(f"{name}:{i}: 行尾有空白")
        if ln != ln.lstrip():
            err(f"{name}:{i}: 行首有空白")
        if ln == "":
            err(f"{name}:{i}: 空行")
    return lines


LATIN = re.compile(r"[A-Za-zÀ-ɏ]")
EMOJI = re.compile("[\U0001F000-\U0001FFFF☀-➿️‍]")
END = "。！？"

sent = {}
for name in ("sentences-a.txt", "sentences-b.txt"):
    lines = read(name)
    sent[name] = lines
    if len(lines) != 100:
        err(f"{name}: 应为 100 行，实际 {len(lines)}")
    for i, ln in enumerate(lines, 1):
        n = len(ln)  # Python str 长度即 Unicode 码点数
        if not 35 <= n <= 48:
            err(f"{name}:{i}: 码点数 {n} 不在 35–48 内: {ln}")
        if "，" not in ln:
            err(f"{name}:{i}: 缺少全角逗号")
        if not re.search(r"[0-9]", ln):
            err(f"{name}:{i}: 缺少 ASCII 数字")
        if ln and ln[-1] not in END:
            err(f"{name}:{i}: 未以 。！？ 结尾")
        if LATIN.search(ln):
            err(f"{name}:{i}: 含拉丁字母")
        if EMOJI.search(ln):
            err(f"{name}:{i}: 含 emoji")
allsent = sent["sentences-a.txt"] + sent["sentences-b.txt"]
if len(set(allsent)) != len(allsent):
    err(f"句子不唯一：{len(allsent)} 行中有 {len(allsent) - len(set(allsent))} 个重复")

# 其他夹具的通用卫生检查与行数范围
RANGES = {
    "mixed-cjk-latin.txt": (25, 35),
    "kinsoku.txt": (25, 35),
    "emoji.txt": (15, 25),
    "combining.txt": (12, 18),
    "rtl.txt": (8, 12),
    "long-paragraph.txt": (1, 1),
}
other = {}
for name, (lo, hi) in RANGES.items():
    lines = read(name)
    other[name] = lines
    if not lo <= len(lines) <= hi:
        err(f"{name}: 行数 {len(lines)} 不在 {lo}–{hi}")

# 内容特征
if not any(re.search(r"[A-Za-z]", l) for l in other["mixed-cjk-latin.txt"]):
    err("mixed-cjk-latin.txt: 没有拉丁字母")
for ch in "、。，．・：；？！）」』】〉》（「『【〈《……——":
    if not any(ch in l for l in other["kinsoku.txt"]):
        err(f"kinsoku.txt: 未覆盖 {ch!r}")
for ch in "ぁぃぅッャュョ":
    if not any(ch in l for l in other["kinsoku.txt"]):
        err(f"kinsoku.txt: 未覆盖小假名 {ch!r}")
em = "\n".join(other["emoji.txt"])
for label, ok in [
    ("ZWJ", "‍" in em),
    ("VS16", "️" in em),
    ("肤色", any(0x1F3FB <= ord(c) <= 0x1F3FF for c in em)),
    ("国旗", any(0x1F1E6 <= ord(c) <= 0x1F1FF for c in em)),
    ("keycap", "⃣" in em),
]:
    if not ok:
        err(f"emoji.txt: 缺少 {label}")
cm = "\n".join(other["combining.txt"])
if not any(unicodedata.combining(c) for c in cm):
    err("combining.txt: 没有组合标记")
if not any(unicodedata.normalize("NFC", l) != l for l in other["combining.txt"]):
    err("combining.txt: 没有非 NFC（分解形式）行")
if not any("฀" <= c <= "๿" for c in cm):
    err("combining.txt: 没有泰文")
if not any("ऀ" <= c <= "ॿ" for c in cm):
    err("combining.txt: 没有天城文")
rt = "\n".join(other["rtl.txt"])
if not any("؀" <= c <= "ۿ" for c in rt):
    err("rtl.txt: 没有阿拉伯文")
if not any("֐" <= c <= "׿" for c in rt):
    err("rtl.txt: 没有希伯来文")
lp = other["long-paragraph.txt"]
if lp and not 1000 <= len(lp[0]) <= 1400:
    err(f"long-paragraph.txt: 长度 {len(lp[0])} 不在 1000–1400")

# manifest
mpath = HERE / "manifest.json"
try:
    m = json.loads(mpath.read_text(encoding="utf-8"))
except Exception as e:
    err(f"manifest.json: 无法解析: {e}")
    m = {}
if m.get("schema") != "tekesui-fixtures/0":
    err("manifest.json: schema 应为 tekesui-fixtures/0")
if m.get("license") != "MIT":
    err("manifest.json: license 应为 MIT")
listed = set()
for e in m.get("fixtures", []):
    for k in ("name", "file", "category", "style", "widths", "scales", "notes"):
        if k not in e:
            err(f"manifest.json: 条目 {e.get('name')} 缺少 {k}")
    f = e.get("file")
    listed.add(f)
    if f and not (HERE / f).is_file():
        err(f"manifest.json: 引用的文件不存在: {f}")
for f in list(RANGES) + ["sentences-a.txt", "sentences-b.txt"]:
    if f not in listed:
        err(f"manifest.json: 未列出 {f}")
for f in ("README.md", "kinsoku.md"):
    if not (HERE / f).is_file():
        err(f"缺少 {f}")

if errors:
    print(f"失败：{len(errors)} 个问题")
    for e in errors:
        print(" -", e)
    sys.exit(1)
print("通过：")
for name in ["sentences-a.txt", "sentences-b.txt", *RANGES]:
    lines = sent.get(name) or other.get(name)
    print(f"  {name}: {len(lines)} 行")
print(f"  句子共 {len(allsent)} 行，唯一 {len(set(allsent))} 行")
