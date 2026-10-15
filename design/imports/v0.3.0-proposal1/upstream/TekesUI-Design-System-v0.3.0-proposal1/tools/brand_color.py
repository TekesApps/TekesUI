"""Small, deterministic sRGB/Oklab conversion helpers for the proposal compiler.

Color equations: CSS Color 4, Oklab/OkLCh and sRGB. The hue-preserving chroma
reduction here is a Tekes policy, NOT an implementation of CSS Local MINDE.
All accessibility comparisons use the emitted, quantized sRGB values.
"""
from __future__ import annotations
import math
import re
from functools import lru_cache


def parse_hex(value: str) -> tuple[float, float, float]:
    if not isinstance(value, str) or not re.fullmatch(r'#[0-9A-Fa-f]{6}', value):
        raise ValueError('BRAND_COLOR_FORMAT: expected opaque sRGB #RRGGBB')
    return tuple(int(value[i:i+2], 16)/255 for i in (1, 3, 5))


def linearize(x: float) -> float:
    return x/12.92 if x <= .04045 else ((x+.055)/1.055)**2.4


def encode(x: float) -> float:
    return 12.92*x if x <= .0031308 else 1.055*x**(1/2.4)-.055


def to_oklab(color: str) -> tuple[float, float, float]:
    r, g, b = map(linearize, parse_hex(color))
    l = (.4122214708*r + .5363325363*g + .0514459929*b)**(1/3)
    m = (.2119034982*r + .6806995451*g + .1073969566*b)**(1/3)
    s = (.0883024619*r + .2817188376*g + .6299787005*b)**(1/3)
    return (.2104542553*l+.7936177850*m-.0040720468*s,
            1.9779984951*l-2.4285922050*m+.4505937099*s,
            .0259040371*l+.7827717662*m-.8086757660*s)


def to_oklch(color: str) -> tuple[float, float, float]:
    l, a, b = to_oklab(color)
    c = math.hypot(a, b)
    # Nearly neutral seeds stay neutral; do not manufacture an arbitrary hue.
    return l, (0. if c < 1e-5 else c), (0. if c < 1e-5 else math.atan2(b, a))


def linear_rgb(l: float, c: float, h: float) -> tuple[float, float, float]:
    a, b = c*math.cos(h), c*math.sin(h)
    ll = (l+.3963377774*a+.2158037573*b)**3
    mm = (l-.1055613458*a-.0638541728*b)**3
    ss = (l-.0894841775*a-1.2914855480*b)**3
    return (4.0767416621*ll-3.3077115913*mm+.2309699292*ss,
            -1.2684380046*ll+2.6097574011*mm-.3413193965*ss,
            -.0041960863*ll-.7034186147*mm+1.7076147010*ss)


def in_gamut(rgb: tuple[float, ...]) -> bool:
    return all(-1e-9 <= x <= 1+1e-9 for x in rgb)


@lru_cache(maxsize=65536)
def tone(l: float, c: float, h: float, iterations: int = 20) -> str:
    if l <= 0: return '#000000'
    if l >= 1: return '#FFFFFF'
    rgb = linear_rgb(l, c, h)
    if not in_gamut(rgb):
        low, high = 0., c
        for _ in range(iterations):
            mid = (low+high)/2
            if in_gamut(linear_rgb(l, mid, h)): low = mid
            else: high = mid
        rgb = linear_rgb(l, low, h)
    # Half-up quantization. The tiny clamp is for floating-point residuals only.
    return '#' + ''.join(f'{int(math.floor(max(0., min(1., encode(x)))*255+.5)):02X}' for x in rgb)


@lru_cache(maxsize=65536)
def luminance(color: str) -> float:
    r, g, b = map(linearize, parse_hex(color))
    return .2126*r+.7152*g+.0722*b


def contrast(a: str, b: str) -> float:
    x, y = sorted((luminance(a), luminance(b)))
    return (y+.05)/(x+.05)


def delta_ok(a: str, b: str) -> float:
    return math.sqrt(sum((x-y)**2 for x,y in zip(to_oklab(a), to_oklab(b))))
