#!/usr/bin/env python3
"""花字透明 PNG 渲染（猫当家 v3.3 实际用的做法，通用化）：双色描边（外深色 + 内白 / 红）+ 竖向渐变填充 + 投影 + 整体旋转。

  python3 huazi_png.py <spec.json> --preset <preset.json> --font-dir <字体目录> --out-dir <输出目录>

preset.json：服务端 `bendi_style {action:"preset", tier}` 的回执原样存下（occ.py bendi style preset <tier> > preset.json）；
  颜色 / 描边 / 阴影 / 旋转范围取它的 huazi，最大宽度取 universal.huazi.maxWidthPx。本机不放风格数据文件。
spec.json：
  {"tier": "hotpot_spicy",                       # 只用来核对与 preset 的档位一致
   "items": [{"name": "hook", "role": "hook", "startSec": 0, "durSec": 3.5, "cy": 820, "rot": -4,
              "lines": [{"segs": [{"text": "猫当家", "font": "primary", "size": 230, "fill": "yellow", "inner": 0}]},
                        {"segs": [{"text": "新派鱼蛙火锅", "font": "secondary", "size": 150, "fill": "white", "inner": 1}],
                         "innerPx": 12, "outerPx": 8}]}]}
  font: primary / secondary（= 档位 huazi.fontPrimary / fontSecondary，文件名见 FONT_FILES）；fill: 档位 fills 里的名字；
  inner: 档位 innerStroke.colors 的下标；innerPx / outerPx 不写用档位值。
输出：每个 item 一张 1080×1920 透明 PNG（<name>.png），以及 huazi.json（每张的非透明 bbox、字体、起止时间），可直接放进 bendi_style check 的 input.huazi。
字体文件：只用识炬内置的 44 种里有开源文件的（OFL）：ZCOOL QingKe HuangYou → Google Fonts「ZCOOL QingKe HuangYou」；Smiley Sans → github.com/atelier-anchor/smiley-sans。字体文件不进技能包，下载到 --font-dir。
"""
import json
import os
import sys

FONT_FILES = {
    "ZCOOL QingKe HuangYou": "ZCOOLQingKeHuangYou-Regular.ttf",
    "Smiley Sans": "SmileySans-Oblique.ttf",
    "ZCOOL KuaiLe": "ZCOOLKuaiLe-Regular.ttf",
    "ZCOOL XiaoWei": "ZCOOLXiaoWei-Regular.ttf",
    "Ma Shan Zheng": "MaShanZheng-Regular.ttf",
    "Noto Serif CJK SC": "NotoSerifCJKsc-Bold.otf",
    "Noto Sans SC": "NotoSansSC-Bold.ttf",
    "LXGW WenKai": "LXGWWenKai-Bold.ttf",
    "Playfair Display": "PlayfairDisplay-Bold.ttf",
}
W, H = 1080, 1920


def _hex(c):
    c = c.lstrip("#")
    return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4))


def tier_style(preset, tier=None):
    """preset = bendi_style preset 的回执：{tier, huazi:{…}, universal:{huazi:{maxWidthPx}}, …}。"""
    if not isinstance(preset, dict) or not isinstance(preset.get("huazi"), dict):
        raise SystemExit("缺 --preset：先 occ.py bendi style preset <档位> > preset.json（服务端 bendi_style 的回执）")
    if tier and preset.get("tier") and tier != preset["tier"]:
        raise SystemExit("spec 的档位 %s 和 preset 的档位 %s 不一致" % (tier, preset["tier"]))
    return preset["huazi"], ((preset.get("universal") or {}).get("huazi") or {}).get("maxWidthPx", 1020)


def render(spec, font_dir, out_dir, preset=None, allow_default_font=False):
    from PIL import Image, ImageDraw, ImageFilter, ImageFont

    hz, max_w = tier_style(preset if preset is not None else spec.get("preset"), spec.get("tier"))
    fam = {"primary": hz["fontPrimary"], "secondary": hz["fontSecondary"]}
    sh = hz["shadow"]
    shadow = (int(sh["dx"]), int(sh["dy"]))

    def font(kind, size):
        name = fam.get(kind, kind)
        path = os.path.join(font_dir or "", FONT_FILES.get(name, name))
        if os.path.isfile(path):
            return ImageFont.truetype(path, size), name
        if allow_default_font:
            return ImageFont.load_default(size), name
        raise SystemExit("缺字体文件 %s（%s）：下载到 --font-dir，见文件头说明" % (path, name))

    def grad(size, c1, c2):
        w, h = size
        g = Image.new("RGBA", size)
        d = ImageDraw.Draw(g)
        for y in range(h):
            t = y / max(h - 1, 1)
            d.line([(0, y), (w, y)], fill=tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3)) + (255,))
        return g

    def line_img(segs, inner, outer):
        fonts = [font(s.get("font", "primary"), int(s["size"]))[0] for s in segs]
        pad = inner + outer + 30
        ws = [f.getbbox(s["text"])[2] for s, f in zip(segs, fonts)]
        hs = [f.getbbox(s["text"])[3] for s, f in zip(segs, fonts)]
        w, h = sum(ws) + pad * 2, max(hs) + pad * 2 + shadow[1]
        base = Image.new("RGBA", (w + shadow[0], h), (0, 0, 0, 0))
        oc = _hex(hz["outerStroke"]["color"]) + (255,)

        def layer(mode):
            L = Image.new("RGBA", base.size, (0, 0, 0, 0))
            x = pad
            for s, f, wi, hi in zip(segs, fonts, ws, hs):
                y = pad + (max(hs) - hi)
                ic_list = hz["innerStroke"]["colors"] or [hz["outerStroke"]["color"]]
                ic = _hex(ic_list[int(s.get("inner", 0)) % len(ic_list)]) + (255,)
                if mode == "outer":
                    ImageDraw.Draw(L).text((x, y), s["text"], font=f, fill=oc, stroke_width=inner + outer, stroke_fill=oc)
                elif mode == "inner" and inner > 0:
                    ImageDraw.Draw(L).text((x, y), s["text"], font=f, fill=ic, stroke_width=inner, stroke_fill=ic)
                elif mode == "fill":
                    c1, c2 = (_hex(c) for c in hz["fills"][s.get("fill") or next(iter(hz["fills"]))])
                    m = Image.new("L", base.size, 0)
                    ImageDraw.Draw(m).text((x, y), s["text"], font=f, fill=255)
                    L = Image.composite(grad(base.size, c1, c2), L, m)
                x += wi
            return L

        out = layer("outer")
        a = out.split()[3].point(lambda v: int(v * float(sh["alpha"])))
        shl = Image.new("RGBA", base.size, (0, 0, 0, 255))
        shl.putalpha(a)
        shd = Image.new("RGBA", base.size, (0, 0, 0, 0))
        shd.paste(shl, shadow, shl)
        shd = shd.filter(ImageFilter.GaussianBlur(int(sh["blur"])))
        img = Image.alpha_composite(base, shd)
        img = Image.alpha_composite(img, out)
        img = Image.alpha_composite(img, layer("inner"))
        return Image.alpha_composite(img, layer("fill"))

    os.makedirs(out_dir, exist_ok=True)
    report = []
    for it in spec["items"]:
        imgs = []
        for ln in it["lines"]:
            imgs.append(line_img(ln["segs"], int(ln.get("innerPx", hz["innerStroke"]["px"])), int(ln.get("outerPx", hz["outerStroke"]["px"]))))
        gap = int(it.get("gap", -10))
        w = max(i.width for i in imgs)
        h = sum(i.height for i in imgs) + gap * (len(imgs) - 1)
        blk = Image.new("RGBA", (w, h), (0, 0, 0, 0))
        y = 0
        for i in imgs:
            blk.alpha_composite(i, ((w - i.width) // 2, y))
            y += i.height + gap
        rot = float(it.get("rot", 0))
        lo, hi = hz["rotationDeg"]
        if not (lo - 1e-6 <= rot <= hi + 1e-6):
            raise SystemExit("%s 旋转 %s° 超出档位 %s–%s°" % (it["name"], rot, lo, hi))
        if rot:
            blk = blk.rotate(rot, resample=Image.BICUBIC, expand=True)
        if blk.width > max_w:
            blk = blk.resize((max_w, int(blk.height * max_w / blk.width)), Image.LANCZOS)
        cv = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        cv.alpha_composite(blk, ((W - blk.width) // 2, int(it["cy"]) - blk.height // 2))
        p = os.path.join(out_dir, it["name"] + ".png")
        cv.save(p)
        fonts = sorted({fam.get(s.get("font", "primary"), s.get("font")) for ln in it["lines"] for s in ln["segs"]})
        report.append({"name": it["name"], "role": it.get("role", "mid"), "png": p,
                       "text": " / ".join("".join(s["text"] for s in ln["segs"]) for ln in it["lines"]),
                       "fonts": fonts, "startSec": it.get("startSec"), "durSec": it.get("durSec"),
                       "bbox": list(cv.getbbox() or (0, 0, 0, 0))})
    with open(os.path.join(out_dir, "huazi.json"), "w", encoding="utf-8") as f:
        json.dump(report, f, ensure_ascii=False, indent=1)
    return report


def main(argv):
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0

    def opt(k, d=None):
        return argv[argv.index(k) + 1] if k in argv else d
    with open(argv[0], encoding="utf-8") as f:
        spec = json.load(f)
    preset = None
    if opt("--preset"):
        with open(opt("--preset"), encoding="utf-8") as f:
            preset = json.load(f)
    rep = render(spec, opt("--font-dir"), opt("--out-dir", "."), preset, "--allow-default-font" in argv)
    print(json.dumps(rep, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
