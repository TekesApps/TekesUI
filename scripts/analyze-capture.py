#!/usr/bin/env python3
"""分析 Gallery 占位页的窗口截图（screencapture -o -l <窗口号> -x，设备像素，不含阴影）。

只用标准库。用法：
    scripts/analyze-capture.py <png> <内容区宽pt> <内容区高pt> [scale，默认 2]

检查：
  1. 图像宽 == 内容宽 * scale；高 - 内容高 * scale 即标题栏的像素高度（2x 下应为 64）；
  2. 左上标记块（20,20 起 40x40 pt）与右下标记块（距右下角各 20pt，40x40 pt）的包围盒，
     换算成 pt 后应与预期一致，这证明内容按新尺寸排版，没有拉伸或停留在旧尺寸；
  3. 线宽：1pt 横线/竖线应为 scale 像素宽，0.5pt 应为 scale/2 像素宽（设备像素）。
布局常量要和 crates/tekes-ui-gallery/src/gallery.rs 保持一致。
"""
import struct
import sys
import zlib

HEADER_H = 44.0
STRIP_X = 260.0
STRIP_Y = 80.0
MARKER = 40.0
INSET = 20.0


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "不是 PNG"
    pos = 8
    idat = []
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            w, h, depth, ctype, _c, _f, interlace = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
    assert depth == 8 and interlace == 0 and ctype in (2, 6), f"不支持的 PNG：depth={depth} ctype={ctype} interlace={interlace}"
    bpp = 4 if ctype == 6 else 3
    raw = zlib.decompress(b"".join(idat))
    stride = w * bpp
    rows = []
    prev = bytearray(stride)
    p = 0
    for _ in range(h):
        ft = raw[p]
        line = bytearray(raw[p + 1 : p + 1 + stride])
        p += 1 + stride
        if ft == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 255
        elif ft == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif ft == 3:
            for i in range(stride):
                left = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 255
        elif ft == 4:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                b = prev[i]
                c = prev[i - bpp] if i >= bpp else 0
                pa = abs(b - c)
                pb = abs(a - c)
                pc = abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        rows.append(line)
        prev = line
    return w, h, bpp, rows


def main():
    if len(sys.argv) < 4:
        print(__doc__)
        sys.exit(2)
    path = sys.argv[1]
    cw, ch = float(sys.argv[2]), float(sys.argv[3])
    scale = float(sys.argv[4]) if len(sys.argv) > 4 else 2.0
    w, h, bpp, rows = read_png(path)

    def px(x, y):
        o = x * bpp
        r = rows[y]
        return (r[o], r[o + 1], r[o + 2])

    ok = True

    def check(name, cond, detail):
        nonlocal ok
        ok &= bool(cond)
        print(f"  [{'OK' if cond else 'FAIL'}] {name}: {detail}")

    print(f"{path}: {w}x{h} px, 预期内容 {cw:g}x{ch:g} pt @ {scale:g}x")
    top = h - round(ch * scale)  # 标题栏占的像素高度，内容从这一行开始
    check("宽度", w == round(cw * scale), f"{w} 对 {round(cw * scale)}")
    check("标题栏像素高度", top > 0, f"{top} px（{top / scale:g} pt）")

    def pt_to_px(x, y):
        return int(round(x * scale)), top + int(round(y * scale))

    def bbox_of(color, tol=2):
        x0 = y0 = 10**9
        x1 = y1 = -1
        for y in range(top, h):
            row = rows[y]
            for x in range(w):
                o = x * bpp
                if abs(row[o] - color[0]) <= tol and abs(row[o + 1] - color[1]) <= tol and abs(row[o + 2] - color[2]) <= tol:
                    x0 = min(x0, x)
                    x1 = max(x1, x)
                    y0 = min(y0, y)
                    y1 = max(y1, y)
        if x1 < 0:
            return None
        # 换算成 pt，原点是内容区左上角；右/下边界取像素右/下沿。
        return ((x0) / scale, (y0 - top) / scale, (x1 + 1) / scale, (y1 + 1 - top) / scale)

    for name, (ex, ey), exp in [
        ("左上标记块", (INSET + MARKER / 2, INSET + MARKER / 2), (INSET, INSET, INSET + MARKER, INSET + MARKER)),
        ("右下标记块", (cw - INSET - MARKER / 2, ch - INSET - MARKER / 2), (cw - INSET - MARKER, ch - INSET - MARKER, cw - INSET, ch - INSET)),
    ]:
        x, y = pt_to_px(ex, ey)
        color = px(min(x, w - 1), min(y, h - 1))
        bb = bbox_of(color)
        if bb is None:
            check(name, False, f"在预期位置的颜色 {color} 找不到包围盒")
            continue
        # 右下块有圆角，包围盒仍是整块；允许 0.5pt 的抗锯齿误差。
        good = all(abs(a - b) <= 0.5 for a, b in zip(bb, exp))
        check(name, good, f"颜色 {color} 包围盒(pt) {tuple(round(v, 2) for v in bb)} 预期 {tuple(round(v, 2) for v in exp)}")

    # 线宽。横线：在 x = STRIP_X + 100 这一列自上而下数连续的非底色像素；竖线：在中间一行自左而右数。
    ys = HEADER_H + STRIP_Y
    bg_x, bg_y = pt_to_px(STRIP_X + 100, ys - 20)
    bg = px(bg_x, bg_y)

    def run_through(points):
        runs = []
        cur = []
        for pix in points:
            if max(abs(pix[i] - bg[i]) for i in range(3)) > 24:
                cur.append(pix)
            elif cur:
                runs.append(cur)
                cur = []
        if cur:
            runs.append(cur)
        return runs

    colx, _ = pt_to_px(STRIP_X + 100, 0)
    y_a = top + int(round((ys - 6) * scale))
    y_b = top + int(round((ys + 20) * scale))
    vruns = run_through([px(colx, y) for y in range(y_a, y_b)])
    print(f"  横线（x={STRIP_X + 100:g}pt 一列）连续像素段：{[len(r) for r in vruns]}，像素值 {[r[0] for r in vruns]}")
    check("1pt 横线宽度", len(vruns) >= 1 and len(vruns[0]) == round(1 * scale), f"{len(vruns[0]) if vruns else None} px，预期 {round(1 * scale)}")
    check("0.5pt 横线宽度", len(vruns) >= 2 and len(vruns[1]) == max(1, round(0.5 * scale)), f"{len(vruns[1]) if len(vruns) > 1 else None} px，预期 {max(1, round(0.5 * scale))}")

    rowy = top + int(round((ys + 30) * scale))
    x_a = int(round((STRIP_X + 215) * scale))
    x_b = int(round((STRIP_X + 245) * scale))
    hruns = run_through([px(x, rowy) for x in range(x_a, x_b)])
    print(f"  竖线（y={ys + 30:g}pt 一行）连续像素段：{[len(r) for r in hruns]}，像素值 {[r[0] for r in hruns]}")
    check("1pt 竖线宽度", len(hruns) >= 1 and len(hruns[0]) == round(1 * scale), f"{len(hruns[0]) if hruns else None} px，预期 {round(1 * scale)}")
    check("0.5pt 竖线宽度", len(hruns) >= 2 and len(hruns[1]) == max(1, round(0.5 * scale)), f"{len(hruns[1]) if len(hruns) > 1 else None} px，预期 {max(1, round(0.5 * scale))}")

    print("结果：", "通过" if ok else "有失败项")
    sys.exit(0 if ok else 1)


main()
