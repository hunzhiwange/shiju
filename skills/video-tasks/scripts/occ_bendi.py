#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""occ_bendi.py —— 本地生活探店/带货统一流程（真人 / AI / 混合）的确定性小工具（草案，供 occ.py 以 `occ.py bendi ...` 转调）。

只处理纯文本 JSON（状态、请求、裁切框），不联网；唯一例外是 style：经 occ.py 调服务端 bendi_style（风格数值在服务端模板，本机不放数据文件）。
读文件的查重命令在 shiju-assets 的 occ_assets.py 里。
状态读写走 MCP：
  occ.py call project_notes '{"action":"get","key":"bendi"}' --out state.json
  occ.py call project_notes @put.json          # {"action":"put","key":"bendi","json":…,"revision":…}

用法:
  occ.py bendi state check <state.json>                 字段与规则校验（按 source 分支：real 不要求角色，有 AI 镜头才要闸门）
  occ.py bendi route <state.json> intake|style|source|real|refpack|beats|assemble|revise   分支点的下一步文件（按 source / shots）
  occ.py bendi gate <state.json> [--shot A1]            角色闸门 + 版本 + 预算，不过则退出码 1（source=real 一律不过）
  occ.py bendi build-request <state.json> --shot A1     组 Wan 3.0 / animate-mix 请求（固定排图顺序 + 图N说明句）并校验
  occ.py bendi lint <request.json>                      单独校验一个请求是否越过 Wan 3.0 限制
  occ.py bendi budget <state.json> [--price 0.6]        生成秒数与预计费用（元/秒，默认 720P 参考价）
  occ.py bendi crop-plan sheetA|sheetB|<行>x<列>          设定页切单视角的 0–1 裁切框（给 media_crop）
  occ.py bendi to-segments <state.json>                 beats → worksheet.segments（给现有模块用）
  occ.py bendi next <state.json> <step 文件>             改 next 字段并打印新状态
  occ.py bendi entry new|revise [<state.json>]          总入口：新做 / 修改探店片 → 该打开的步骤文件
  occ.py bendi revise-route <反馈类型>                   修改探店片：反馈类型 → 步骤或属性补丁（layout/volume → fine-tune-short-path）
  occ.py bendi style detect "<店名 品类 招牌菜>" | preset <档位>
  occ.py bendi style check|beats|snap|sip|bgm-score <input.json> [--asset <BGM 素材 id>] [--skill-version <v>]
                                                        风格预设与审美机检 = 服务端 bendi_style（input.json 即工具的 input；beats / snap 可带 --asset 用 media_beats）
人物（形象、设定页、建档、授权）归本技能 model 模块（modules/model.md；机检在同目录 occ_model.py，state check 直接调它的 check_character）。
素材命令（范围、sha1、指纹、查重、入库记录）归 shiju-assets：occ.py bendi <命令> 原样转给 ../shiju-assets/scripts/occ_assets.py
  scope-for · classify-scope · hash-file · fingerprint · fp-compare · dedupe · fp-tags · check-uploads
退出码: 0 通过；1 规则不过（打印原因）；2 用法错误；style 另有 3 传输错误
"""
import json
import sys

LIMITS = {"ref_image": 10, "ref_video": 5, "ref_audio": 5, "ref_video_sec": 15.0,
          "ref_audio_sec": 15.0, "dur_min": 2, "dur_max": 30, "io_max": 30.0, "total_media": 20}
VIEW_ROLES = ["face_front", "face_34", "face_side", "half_front", "half_34",
              "full_front", "full_34", "full_side", "full_back",
              "expr_talk", "expr_smile", "expr_taste", "expr_surprise", "accessory"]
VIEW_LABEL = {"face_front": "正脸特写", "face_34": "3/4 侧脸", "face_side": "侧脸",
              "half_front": "正面半身", "half_34": "3/4 半身", "full_front": "正面全身",
              "full_34": "3/4 全身", "full_side": "侧面全身", "full_back": "背面全身",
              "expr_talk": "说话表情", "expr_smile": "微笑", "expr_taste": "试吃惊喜表情",
              "expr_surprise": "惊讶", "accessory": "配饰特写"}
REQUIRED_VIEWS = ["face_front", "face_34", "half_front", "half_34", "full_front", "full_side", "full_back"]
ENV_LABEL = {"storefront": "门头", "entrance": "入口", "table_seat": "用餐桌位", "table_reverse": "桌位反打",
             "wide": "店内全景", "counter": "吧台/明档", "decor": "装饰墙/打卡点", "kitchen": "后厨"}
# 入库范围、sha1、视频指纹、查重、入库记录校验归 shiju-assets 技能所有（../shiju-assets/scripts/occ_assets.py），这里只引用
import os
# 并排安装（skills/shiju-assets）；本技能被软链安装时再按真实路径找一次
for _here in (os.path.abspath(__file__), os.path.realpath(__file__)):
    _ASSETS = os.path.normpath(os.path.join(os.path.dirname(_here), "..", "..", "shiju-assets", "scripts"))
    if os.path.isfile(os.path.join(_ASSETS, "occ_assets.py")):
        sys.path.insert(0, _ASSETS)
        break
# shiju-assets 未安装时不让整个 bendi 崩（ModuleNotFoundError）：状态/计划/裁切等纯文本命令照常可用，
# 素材命令给出安装提示，validate 跳过入库记录校验并提示。
try:
    import occ_assets  # noqa: E402
    from occ_assets import (SCOPE_BY_CATEGORY, REAL_CATEGORIES, scope_for, classify_scope, upload_scope,  # noqa: E402,F401
                            hash_file, fingerprint, fp_compare, fp_pack, dedupe, fp_tags, check_uploads)
except ImportError:  # pragma: no cover - exercised by test_occ_bendi via a stripped sys.path
    occ_assets = None
    ASSETS_MISSING = ("未找到 shiju-assets 技能（../shiju-assets/scripts/occ_assets.py）：素材范围/sha1/指纹/查重/入库记录命令不可用，"
                      "请把 shiju-assets 与 cutvideo 并排安装")

    def check_uploads(uploads, gen_ids):  # noqa: D401
        if uploads:
            sys.stderr.write("[bendi] 跳过入库记录校验：" + ASSETS_MISSING + "\n")
        return []

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
# 人物档案校验：本技能 model 模块（modules/model.md），同目录 occ_model.py
import occ_model  # noqa: E402

MODES = ("R", "K", "F", "REPLACE", "EDIT")
# 画面里出现这些词 = 有真实物品入画，必须带参考包 P 帧（门店真实菜品 / 饮料 / 商品），不许模型臆造（猫当家 v3.2 饮料用错的教训）
PROP_WORDS = ("饮料", "饮品", "果汁", "果饮", "奶茶", "咖啡", "酒", "瓶", "杯", "菜", "锅", "甜品", "蛋糕", "面包", "商品", "产品")
SIP_WORDS = ("喝", "吸一口", "抿")
# 视频主轨固定后缀（防烧字幕）和照抄参考句：见 references/realism-prompts.md §4
NO_TEXT_SUFFIX = ("Clean raw camera footage only. Spoken dialogue exists in the audio track only. "
                  "No burned-in captions, no subtitles, no on-screen text, no typography, no graphic overlays, "
                  "no stickers, no UI elements in any frame.")
# 视频写实结尾句：猫当家 G1 / S1–S5 / S5b 每条都用的原句（build_character_scene 自动补）
VIDEO_REAL_TAIL = "画面自然写实，皮肤有真实纹理和细小毛孔，淡妆，五官比例协调，双手各五根手指，动作连贯自然，人体结构正确，曝光正常，色彩自然不过饱和。"
COPY_REF = "提示词没写到的、参考图里已有的人物、服装、配饰和门店环境直接照抄参考图；运镜、机位、景别严格参考；手机实拍感，有呼吸感，不自行脑补内容"
STILL_ONLY = ["85mm", "f/1.2", "f1.2"]  # 只用于定妆照 / 三视图，视频主轨保持手机深景深
BANNED = ["8K", "8k", "电影级", "大师", "masterpiece", "flawless", "完美无瑕", "像明星", "长得像", "亲测", "真人实测"]


def load(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError) as e:
        die("读不了 %s：%s" % (path, e), 2)


def die(msg, code=1):
    sys.stderr.write(msg + "\n")
    sys.exit(code)


def fail(errors):
    for e in errors:
        print("✗ " + e)
    sys.exit(1 if errors else 0)


SOURCES = ("real", "ai", "mixed")
RHYTHM_BY_SOURCE = {"real": ("footage",), "ai": ("reference", "footage", "mixed"), "mixed": ("mixed", "footage")}
STEP = "flows/bendi/steps/"


def has_ai(st):
    """有没有 AI 镜头：source 为 ai/mixed，或已有 shots / generated。只有这时才要求角色闸门和 AI 步骤。"""
    return st.get("source") in ("ai", "mixed") or bool(st.get("shots")) or bool(st.get("generated"))


def route(st, after):
    """分支点的确定性路由。after=intake|source|real|refpack|beats，返回下一个步骤文件。"""
    src = st.get("source")
    if after == "intake":
        return STEP + "step-01a-style.md"
    if after == "style":
        return STEP + "step-02-source.md"
    if after == "assemble":
        return STEP + "step-05a-voice.md"
    if after == "revise":
        return STEP + "step-07-revise.md"
    if after == "source":
        return STEP + ("step-02a-character.md" if src == "ai" else "step-02r-real.md")
    if after == "real":
        return STEP + ("step-02a-character.md" if src == "mixed" else "step-03-beat-plan.md")
    if after == "refpack":
        return STEP + "step-03-beat-plan.md"
    if after == "beats":
        return STEP + ("step-04-generate-aroll.md" if st.get("shots") else "step-05-assemble.md")
    raise ValueError("after 只能是 intake/style/source/real/refpack/beats/assemble/revise")


ENTRY_NEW = STEP + "step-00-library.md"
# 修改已有探店片：反馈类型 → 从哪一步重做（之后的步骤按依赖顺延，见 step-07-revise.md）
# 仅位置/音量类走属性补丁短路径（references/fine-tune-short-path.md），不进 5d→6 全链路
REVISE_PATCH = "references/fine-tune-short-path.md"
REVISE_PATCH_KINDS = ("layout", "caption_layout", "volume", "track_volume")
REVISE_MAP = {
    "bgm": "step-05b-bgm.md", "rhythm": "step-05c-beat-sync.md", "beat": "step-05c-beat-sync.md",
    "voice": "step-05a-voice.md", "pronunciation": "step-05a-voice.md",
    "text": "step-05d-text.md", "font": "step-05d-text.md", "caption": "step-05d-text.md", "huazi": "step-05d-text.md",
    "badge": "step-06-qa-deliver.md", "loudness": "step-06-qa-deliver.md",
    "drink": "step-04-generate-aroll.md", "dish": "step-04-generate-aroll.md", "action": "step-04-generate-aroll.md",
    "outfit": "step-02a-character.md", "face": "step-02a-character.md",
    "structure": "step-03-beat-plan.md", "shots": "step-03-beat-plan.md", "style": "step-01a-style.md",
}


def entry(intent, st=None):
    """总入口：新做探店片 → 步骤 0；修改已有探店片（工程里有 bendi 状态）→ 步骤 7；状态里有未完成的 next → 续跑。"""
    if intent == "new":
        return ENTRY_NEW
    if intent == "revise":
        if st and st.get("next") and "step-07" not in st["next"]:
            return st["next"]  # 上一版还没做完：先续跑，不当修改
        return STEP + "step-07-revise.md"
    raise ValueError("intent 只能是 new / revise")


def revise_route(kind):
    k = (kind or "").lower()
    if k in REVISE_PATCH_KINDS:
        return REVISE_PATCH
    if k not in REVISE_MAP:
        kinds = sorted(set(REVISE_MAP) | set(REVISE_PATCH_KINDS))
        raise ValueError("反馈类型只能是 %s" % "/".join(kinds))
    return STEP + REVISE_MAP[k]


def check_state(st):
    errs = []
    src = st.get("source")
    for k in ("next", "source", "subgenre", "beats"):
        if k not in st:
            errs.append("缺字段 %s" % k)
    if "entry" in st:
        errs.append("旧字段 entry 已由 source（real/ai/mixed）取代")
    if src not in (None,) + SOURCES:
        errs.append("source 只能是 real/ai/mixed")
    ai = has_ai(st)
    if src == "real":
        if st.get("shots") or st.get("generated"):
            errs.append("source=real 不能有 AI 镜头（shots/generated）；要 AI 补镜就改 mixed")
        if any(n in (st.get("next") or "") for n in ("step-02a-", "step-02b-", "step-04-")):
            errs.append("source=real 不走 AI 步骤（2a/2b/4），next=%s" % st.get("next"))
    if st.get("host") == "none" and src != "real":
        errs.append("host=none（无人出镜）只用于 source=real")
    if src in ("real", "mixed") and st.get("beats") and not st.get("realSegments") and st.get("host") != "none":
        errs.append("source=%s 排节拍前要有 realSegments（步骤 2r 锁定的真人段）" % src)
    if ai and "character" not in st:
        errs.append("有 AI 镜头却缺 character（步骤 2a 角色闸门）")
    if ai and st.get("shots") and not (st.get("budget") or {}).get("approvedSec") \
            and any(s.get("status") not in (None, "todo") for s in st.get("shots", [])):
        errs.append("AI 镜头已提交但 budget.approvedSec 为空")
    rm = st.get("rhythmMode")
    if rm is not None and rm not in ("footage", "reference", "mixed"):
        errs.append("rhythmMode 只能是 footage/reference/mixed")
    if rm and src in RHYTHM_BY_SOURCE and rm not in RHYTHM_BY_SOURCE[src]:
        errs.append("source=%s 时 rhythmMode 只能是 %s（现在 %s）" % (src, "/".join(RHYTHM_BY_SOURCE[src]), rm))
    real_rng = {(r.get("sourceAssetId"), tuple(r.get("range") or [])) for r in st.get("realSegments", [])}
    for b_ in st.get("beats", []):
        src_b = b_.get("rhythmSource")
        if b_.get("track") == "A" and rm and not src_b:
            errs.append("A 拍 %s 没写 rhythmSource" % b_.get("id"))
        if src_b == "footage":
            rg = b_.get("range")
            if not b_.get("sourceAssetId") or not rg:
                errs.append("原片拍 %s 要写 sourceAssetId 和 range（时长跟原片）" % b_.get("id"))
            elif b_.get("budgetSec") and abs((rg[1] - rg[0]) - b_["budgetSec"]) > 0.3:
                errs.append("原片拍 %s 的 budgetSec 与原片区间长度不符：不为凑爆款时长截断或拉伸真人段" % b_.get("id"))
            elif b_.get("shotType") != "replace" and real_rng and (b_.get("sourceAssetId"), tuple(rg)) not in real_rng:
                errs.append("真人拍 %s 的区间不在 realSegments 里（步骤 3 不改真人段长短）" % b_.get("id"))
        if src_b == "reference" and src == "real":
            errs.append("source=real 的拍 %s 不能按爆款定时长" % b_.get("id"))
        if rm == "footage" and src_b == "reference":
            errs.append("rhythmMode=footage 却有按爆款定时长的拍 %s；应改 mixed" % b_.get("id"))
    if st.get("entryCase") not in ("library", "reference_only", "nothing"):
        errs.append("entryCase 只能是 library/reference_only/nothing（步骤 0 查库后、步骤 1 定）")
    lib = st.get("library") or {}
    if not lib.get("status"):
        errs.append("缺 library.status：步骤 0 第一件事是查私域素材库")
    for a in st.get("inventory", []):
        if a.get("source") == "private" and not a.get("assetRef"):
            errs.append("私域素材 %s 缺 assetRef" % a.get("assetId"))
    gen_ids = {x.get("resultAssetId") for x in st.get("shots", []) if x.get("resultAssetId")}
    gen_ids |= {g.get("assetId") for g in st.get("generated", []) if g.get("assetId")}
    errs += check_uploads(st.get("uploads", []), gen_ids)
    if (st.get("reference") or {}).get("source") == "text" and st.get("entryCase") == "nothing":
        errs.append("entryCase=nothing 却有文字参考：应改为 reference_only")
    ch = st.get("character") or {}
    roles = {v.get("role") for v in ch.get("views", [])}
    if ai and ch.get("status") == "confirmed":
        miss = [r for r in REQUIRED_VIEWS if r not in roles]
        if miss:
            errs.append("角色已确认但缺单视角：%s" % ",".join(miss))
        if not ch.get("confirmedBy"):
            errs.append("character.confirmedBy 要记用户原话")
    sty = st.get("style") or {}
    nxt = st.get("next") or ""
    early = any(x in nxt for x in ("step-00-", "step-01-", "step-01a-"))
    if sty:  # 档位是否存在由服务端 bendi_style 判断（公司可覆盖档位表）；这里只查填了
        if not isinstance(sty.get("tier"), str) or not sty["tier"].strip():
            errs.append("style.tier 为空（bendi_style {action:\"detect\"} 得到档位）")
    elif not sty and (st.get("beats") or (nxt and not early)):
        errs.append("缺 style.tier：步骤 1a 先判断探店类型、加载风格预设")
    if ai and ch.get("status") == "confirmed":
        errs += ["character: " + e for e in occ_model.check_character(ch)]
    kinds = {a.get("assetId"): a.get("kind") for a in st.get("inventory", [])}
    for b in st.get("beats", []):
        if b.get("track") == "B" and not b.get("brollAssetId"):
            errs.append("B 拍 %s 没有 brollAssetId" % b.get("id"))
        if b.get("track") == "B" and b.get("mode"):
            errs.append("B 拍 %s 不应有生成 mode" % b.get("id"))
    beat_ids = {b.get("id") for b in st.get("beats", [])}
    for s in st.get("shots", []):
        if s.get("mode") not in MODES:
            errs.append("镜头 %s 的 mode 不合法" % s.get("id"))
        if s.get("beat") not in beat_ids:
            errs.append("镜头 %s 指向不存在的拍 %s" % (s.get("id"), s.get("beat")))
        for ref in s.get("props", []):
            if kinds.get(ref) in ("dish", "product"):
                errs.append("镜头 %s 直接引用了实拍菜品/产品视频，应引用参考包 P 帧" % s.get("id"))
    return errs


def gate_errors(st, shot=None):
    errs = []
    if st.get("source") == "real":
        return ["source=real：没有 AI 镜头，不该调用生成；要补镜先改 mixed 并过步骤 2a"]
    ch = st.get("character") or {}
    if ch.get("status") != "confirmed":
        errs.append("角色闸门：character.status=%s，回步骤 2a" % ch.get("status"))
    b = st.get("budget") or {}
    if not b.get("approvedSec"):
        errs.append("预算未授权：budget.approvedSec 为空，回步骤 3")
    elif b.get("usedSec", 0) >= b["approvedSec"]:
        errs.append("预算已用完：usedSec=%s" % b.get("usedSec"))
    shots = st.get("shots", [])
    if shot:
        shots = [s for s in shots if s.get("id") == shot]
        if not shots:
            errs.append("没有镜头 %s" % shot)
    for s in shots:
        if s.get("characterVersion") != ch.get("version"):
            errs.append("镜头 %s 的 characterVersion 与当前角色 v%s 不同，重开为 todo" % (s.get("id"), ch.get("version")))
    return errs


def _find(items, key, val):
    for it in items:
        if it.get(key) == val:
            return it
    return None


def build_request(st, shot_id):
    s = _find(st.get("shots", []), "id", shot_id)
    if not s:
        die("没有镜头 %s" % shot_id, 2)
    ch, rp = st["character"], st.get("refpack", {})
    mode = s.get("mode")
    name = ch.get("name", "角色")
    if s.get("kind") == "character_scene":
        return build_character_scene(st, s)
    if mode == "REPLACE":
        role = s.get("identity", ["half_front"])[0]
        v = _find(ch["views"], "role", role)
        return {"tool": "video_character_replace", "args": {
            "model": "wan2.2-animate-mix", "imageUrl": v and v.get("url"),
            "videoUrl": (s.get("sourceClip") or {}).get("url"),
            "mode": s.get("quality", "wan-pro")},
            "notes": ["原片人物须从第一帧就在画面里；片长 2–30 秒；只用 1 张身份图（%s）" % VIEW_LABEL.get(role, role)]}
    media, lines, n = [], [], 0
    if mode == "F":
        for k in ("firstFrame", "lastFrame"):
            fr = s.get(k)
            if fr:
                media.append({"type": "first_frame" if k == "firstFrame" else "last_frame", "url": fr.get("url")})
        prompt = s.get("prompt", "")
    else:
        idn = []
        for role in s.get("identity", []):
            v = _find(ch["views"], "role", role)
            if not v:
                die("角色没有视角 %s" % role, 1)
            n += 1
            media.append({"type": "reference_image", "url": v.get("url")})
            idn.append("图%d" % n)
            label = VIEW_LABEL.get(role, role)
            lines.append("图%d 是%s的%s" % (n, name, label))
        if idn:
            lines.append("%s 是同一个人%s，全程锁定她的脸型、五官、发型、肤色和服装，不要改变" % ("、".join(idn), name))
        for eid in s.get("env", []):
            e = _find(rp.get("env", []), "id", eid)
            if not e:
                die("参考包没有环境 %s" % eid, 1)
            n += 1
            media.append({"type": "reference_image", "url": e.get("url")})
            lines.append("图%d 是本店真实环境（%s），只参考空间布局、陈设与光线，不参考其中的人物" % (n, ENV_LABEL.get(e.get("role"), e.get("role"))))
        for pid in s.get("props", []):
            p = _find(rp.get("props", []), "id", pid)
            if not p:
                die("参考包没有菜品/产品 %s" % pid, 1)
            n += 1
            media.append({"type": "reference_image", "url": p.get("url")})
            lines.append("图%d 是桌上真实的%s，只用于保持桌面一致，放在画面边缘，不作为画面主体" % (n, p.get("dish") or p.get("role")))
        for cid in s.get("comp", []):
            c = _find(rp.get("comp", []), "id", cid)
            if c and c.get("asImage"):
                n += 1
                media.append({"type": "reference_image", "url": c.get("url")})
                lines.append("图%d 只参考构图、机位和姿势，不参考人物长相、服装和菜品" % n)
            elif c:
                lines.append("构图：%s" % c.get("note"))
        kf = s.get("keyframe")
        if kf:
            n += 1
            media.append({"type": "reference_image", "url": kf.get("url")})
            lines.append("图%d 是本镜头的关键帧，画面构图、人物位置和桌面布置参考图%d" % (n, n))
        rv = s.get("refVideo")
        if rv:
            media.append({"type": "reference_video", "url": rv.get("url"), "durationSec": rv.get("durationSec")})
        au = s.get("audio")
        if au:
            media.append({"type": "reference_audio", "url": au.get("url"), "durationSec": au.get("durationSec")})
            lines.append("%s按音频1的内容和音色说话，口型同步" % name)
        if n and mode != "EDIT":
            lines.append(COPY_REF)
        prompt = "；".join(lines) + "。\n" + s.get("prompt", "")
    if NO_TEXT_SUFFIX not in prompt:
        prompt = prompt.rstrip() + "\n" + NO_TEXT_SUFFIX
    if mode == "EDIT" and not prompt.lstrip().startswith("编辑视频1"):
        prompt = "编辑视频1：" + prompt
    req = {"model": s.get("model", "wan3.0-video"),
           "input": {"prompt": prompt, "media": media},
           "parameters": {"resolution": s.get("resolution", "720P"), "ratio": s.get("ratio", "9:16"),
                          "duration": -1 if mode == "EDIT" else s.get("durationSec", 5),
                          "audio": False if mode == "F" else bool(s.get("audio")) or s.get("nativeAudio", False),
                          "prompt_extend": True, "watermark": False}}
    if s.get("seed") is not None:
        req["parameters"]["seed"] = s["seed"]
    return {"tool": "video_generate", "args": {"provider": "wan3", "request": req}}


def build_character_scene(st, s):
    """猫当家 v2–v3.3 实际用法：kind:"character_scene" + characterId/characterVersion，服务端把角色档案 views 按档案顺序
    放在最前面当 图1…图N（不用自己传身份图），参考包的环境 / 实物图从 图N+1 起编号，台词音频是 音频1。"""
    ch, rp = st["character"], st.get("refpack", {})
    name = ch.get("name", "主角")
    nv = int(ch.get("viewCount") or len(ch.get("views") or []))
    if nv < 2:
        die("character_scene 需要角色档案里至少 2 张单视角（model 模块产出 views）", 1)
    lines = ["这是一条%s探店vlog的%s" % (s.get("storeType", ""), s.get("shotLabel", "镜头")),
             "主角是图1到图%d中的同一位年轻女生%s：%s；脸型、五官、发型、肤色、服装与图1到图%d完全一致" % (nv, name, ch.get("lookLine", ""), nv)]
    media, n = [], nv
    for kind, key, fmt in (("env", "env", "图%d是%s"), ("props", "props", "图%d是%s")):
        for rid in s.get(key, []):
            r = _find(rp.get(key, []), "id", rid)
            if not r:
                die("参考包没有 %s" % rid, 1)
            n += 1
            media.append({"type": "reference_image", "assetRef": r.get("assetRef"), "url": r.get("url")})
            lines.append(fmt % (n, r.get("say") or r.get("note") or ENV_LABEL.get(r.get("role"), r.get("role"))))
    au = s.get("audio")
    if au:
        media.append({"type": "reference_audio", "assetRef": au.get("assetRef"), "url": au.get("url"), "durationSec": au.get("durationSec")})
    body = s.get("prompt", "")
    if "画面自然写实" not in body:
        body = body.rstrip() + VIDEO_REAL_TAIL
    prompt = "。".join(lines) + "。" + COPY_REF + "。" + body
    if NO_TEXT_SUFFIX not in prompt:
        prompt = prompt.rstrip() + " " + NO_TEXT_SUFFIX
    args = {"prompt": prompt, "kind": "character_scene", "provider": "wan3", "model": s.get("model", "wan3.0-video"),
            "characterId": ch.get("characterId"), "characterVersion": ch.get("version"),
            "media": [{k: v for k, v in m.items() if v is not None} for m in media],
            "durationSec": s.get("durationSec", 5), "resolution": s.get("resolution", "720P"), "ratio": "9:16",
            "audio": bool(au), "viewCount": nv}
    if s.get("seed") is not None:
        args["seed"] = s["seed"]
    if s.get("idempotencyKey"):
        args["idempotencyKey"] = s["idempotencyKey"]
    return {"tool": "video_generate", "args": args}


def lint_character_scene(a):
    errs = []
    nv = int(a.get("viewCount") or 0)
    media = a.get("media") or []
    imgs = [m for m in media if m.get("type") == "reference_image"]
    auds = [m for m in media if m.get("type") == "reference_audio"]
    if not a.get("characterId") or a.get("characterVersion") is None:
        errs.append("character_scene 要带 characterId + characterVersion（model 模块产出、已确认的版本）")
    if nv + len(imgs) > LIMITS["ref_image"]:
        errs.append("角色视图 %d + 参考图 %d > 10 张" % (nv, len(imgs)))
    if sum(float(m.get("durationSec") or 0) for m in auds) > LIMITS["ref_audio_sec"]:
        errs.append("参考音频超过 15 秒：按场景拆成 ≤15 秒一段")
    d = a.get("durationSec", 5)
    if not (LIMITS["dur_min"] <= d <= LIMITS["dur_max"]):
        errs.append("时长 %s 不在 2–30 秒" % d)
    pr = a.get("prompt", "")
    for k in range(nv + 1, nv + len(imgs) + 1):
        if ("图%d" % k) not in pr:
            errs.append("提示词没说明 图%d 是什么（角色视图占 图1–图%d）" % (k, nv))
    if a.get("audio") and "音频1" not in pr:
        errs.append("带了台词音频却没写“按音频1说话，口型同步”")
    if any(w in pr for w in PROP_WORDS) and not imgs:
        errs.append("画面里有菜品 / 饮料 / 商品却没带门店实物参考图（参考包 P 帧）：不许让模型臆造")
    if any(w in pr for w in SIP_WORDS) and any(w in pr for w in ("饮料", "饮品", "果汁", "果饮", "奶茶", "咖啡", "瓶", "杯")):
        if "吸管" not in pr:
            errs.append("喝饮料镜头要写“用吸管喝”，并把吸的时段和台词错开（松开吸管后再说）")
    for w in BANNED:
        if w in pr:
            errs.append("提示词含禁用写法：%s" % w)
    if any(w in pr.lower() for w in STILL_ONLY):
        errs.append("视频提示词含 85mm / f1.2：只用于定妆照，视频要手机深景深")
    if NO_TEXT_SUFFIX not in pr and "No burned-in captions" not in pr:
        errs.append("缺防字幕后缀（No burned-in captions…）")
    if "画面自然写实" not in pr or "五根手指" not in pr:
        errs.append("缺写实结尾句（VIDEO_REAL_TAIL：皮肤纹理、双手各五根手指、人体结构正确…）")
    return errs


def lint_request(req):
    errs = []
    if (req.get("args") or {}).get("kind") == "character_scene":
        return lint_character_scene(req["args"])
    if "request" in req.get("args", {}):
        req = req["args"]["request"]
    elif req.get("tool") == "video_character_replace":
        a = req.get("args", {})
        return [m for m, ok in (("缺 imageUrl", a.get("imageUrl")), ("缺 videoUrl（先 media_clip 截 2–30 秒）", a.get("videoUrl"))) if not ok]
    media = req.get("input", {}).get("media", [])
    par = req.get("parameters", {})
    cnt = {}
    for m in media:
        cnt[m.get("type")] = cnt.get(m.get("type"), 0) + 1
    refs = sum(v for k, v in cnt.items() if k.startswith("reference_") or k in ("file", "link"))
    frames = cnt.get("first_frame", 0) + cnt.get("last_frame", 0)
    if refs and frames:
        errs.append("first/last_frame 与 reference_* 不能同一请求（Wan 3.0 互斥）")
    if cnt.get("first_frame", 0) > 1 or cnt.get("last_frame", 0) > 1:
        errs.append("首帧、尾帧各最多 1 张")
    if frames and par.get("audio"):
        errs.append("首尾帧模式不能带音频：audio 设 false，台词走画外音")
    if cnt.get("reference_image", 0) > LIMITS["ref_image"]:
        errs.append("参考图 %d 张，超过 10" % cnt["reference_image"])
    if cnt.get("reference_video", 0) > LIMITS["ref_video"] or cnt.get("reference_audio", 0) > LIMITS["ref_audio"]:
        errs.append("参考视频/音频各最多 5 个")
    if len(media) > LIMITS["total_media"]:
        errs.append("素材总数超过 20")
    vsec = sum(float(m.get("durationSec") or 0) for m in media if m.get("type") == "reference_video")
    asec = sum(float(m.get("durationSec") or 0) for m in media if m.get("type") == "reference_audio")
    for m in media:
        if m.get("type") in ("reference_video", "reference_audio") and not m.get("durationSec"):
            errs.append("%s 没写 durationSec，无法核 15 秒上限" % m.get("type"))
        if not m.get("url"):
            errs.append("%s 缺 url（过期了就 asset_get 重新取）" % m.get("type"))
    if vsec > LIMITS["ref_video_sec"]:
        errs.append("参考视频总长 %.1f 秒，超过 15" % vsec)
    if asec > LIMITS["ref_audio_sec"]:
        errs.append("参考音频总长 %.1f 秒，超过 15" % asec)
    d = par.get("duration", 5)
    if d != -1 and not (LIMITS["dur_min"] <= d <= LIMITS["dur_max"]):
        errs.append("时长 %s 不在 2–30 秒" % d)
    if vsec and d != -1 and vsec + d > LIMITS["io_max"]:
        errs.append("有视频输入时输入 + 输出 ≤30 秒（现在 %.1f）" % (vsec + d))
    prompt = req.get("input", {}).get("prompt", "")
    n_img = cnt.get("reference_image", 0)
    for i in range(1, n_img + 1):
        if n_img >= 2 and ("图%d" % i) not in prompt:
            errs.append("提示词没提到 图%d（≥2 张图必须逐张编号说明）" % i)
    for w in BANNED:
        if w in prompt:
            errs.append("提示词含禁用写法：%s" % w)
    low = prompt.lower()
    for w in STILL_ONLY:
        if w in low:
            errs.append("视频提示词含 %s：85mm / f1.2 大光圈只用于定妆照和三视图，视频主轨要手机实拍深景深" % w)
    if prompt and NO_TEXT_SUFFIX not in prompt:
        errs.append("缺防字幕后缀（No burned-in captions…），用 build-request 生成或照 realism-prompts.md §4 补上")
    return errs


def budget(st, price):
    gen = [s for s in st.get("shots", []) if s.get("mode") in MODES]
    sec = sum(float(s.get("durationSec") or 5) for s in gen)
    est = sec * price
    return {"shots": len(gen), "genSec": sec, "pricePerSec": price, "estimate": round(est, 1),
            "withRetry": round(est * 1.5, 1), "note": "单价以阿里云百炼官方价目为准；换人(animate-mix)另计"}


def crop_plan(kind):
    if kind == "sheetA":
        top = ["full_front", "full_34", "full_side", "full_back"]
        bot = ["face_front", "face_34", "face_side"]
        out = [{"role": r, "box": {"x": round(i / 4, 4), "y": 0, "w": 0.25, "h": 0.62}} for i, r in enumerate(top)]
        out += [{"role": r, "box": {"x": round(i / 3, 4), "y": 0.62, "w": round(1 / 3, 4), "h": 0.38}} for i, r in enumerate(bot)]
        return out
    if kind == "sheetB":
        names = ["expr_talk", "expr_smile", "expr_taste", "expr_satisfied", "expr_think",
                 "expr_surprise", "expr_point", "expr_toast", "expr_lookdown"]
        return [{"role": names[i], "box": {"x": round((i % 3) / 3, 4), "y": round((i // 3) / 3, 4),
                                           "w": round(1 / 3, 4), "h": round(1 / 3, 4)}} for i in range(9)]
    try:
        r, c = [int(x) for x in kind.lower().split("x")]
    except ValueError:
        die("crop-plan 参数：sheetA | sheetB | <行>x<列>", 2)
    return [{"cell": "%d-%d" % (i // c + 1, i % c + 1), "box": {"x": round((i % c) / c, 4), "y": round((i // c) / r, 4),
             "w": round(1 / c, 4), "h": round(1 / r, 4)}} for i in range(r * c)]


def to_segments(st):
    segs = []
    for b in st.get("beats", []):
        a = b.get("track") == "A"
        real = a and b.get("rhythmSource") == "footage" and b.get("shotType") != "replace"
        segs.append({"id": b.get("id"), "role": b.get("role"), "says": b.get("says"),
                     "mainLine": "host" if a else ("tts" if b.get("says") else "bed"),
                     "visualMode": "real" if (real or not a) else "ai_daren",
                     "references": [b.get("sourceAssetId")] if real else ([] if a else [b.get("brollAssetId")]),
                     "range": b.get("range"),
                     "startState": (b.get("handoff") or {}).get("in"),
                     "endState": (b.get("handoff") or {}).get("out"),
                     "outputs": True})
    return segs


STYLE_FILE_ACTIONS = {"check": "check", "beats": "beats", "snap": "snap", "sip": "sip", "bgm-score": "bgm_score"}


def style_args(rest):
    """命令行 → bendi_style 参数；用法错误抛 ValueError。"""
    rest = list(rest)

    def take(flag):
        if flag in rest:
            i = rest.index(flag)
            if i + 1 >= len(rest):
                raise ValueError("%s 需要一个值" % flag)
            v = rest[i + 1]
            del rest[i:i + 2]
            return v
        return None
    asset, version = take("--asset"), take("--skill-version")
    if not rest:
        raise ValueError("用法：occ.py bendi style detect|preset|check|beats|snap|sip|bgm-score …")
    sub, rest = rest[0], rest[1:]
    if sub == "detect" and rest:
        args = {"action": "detect", "text": " ".join(rest)}
    elif sub == "preset" and len(rest) == 1:
        args = {"action": "preset", "tier": rest[0]}
    elif sub in STYLE_FILE_ACTIONS and len(rest) == 1:
        inp = load(rest[0])
        if not isinstance(inp, dict):
            raise ValueError("%s 要一个 JSON 对象" % rest[0])
        args = {"action": STYLE_FILE_ACTIONS[sub], "input": inp}
        if sub in ("beats", "snap"):
            asset = asset or inp.pop("assetId", None)
            if asset:
                args["assetId"] = asset
    else:
        raise ValueError("用法：occ.py bendi style detect \"<文本>\" | preset <档位> | check|beats|snap|sip|bgm-score <input.json> [--asset ID]")
    if version:
        args["skillVersion"] = version
    return args


def style(rest, call, sleep=None, max_polls=36):
    """调服务端 bendi_style；media_beats 没算完（status:running）时每 5 秒同参数重调，最多约 3 分钟。"""
    try:
        args = style_args(rest)
    except ValueError as e:
        die(str(e), 2)
    if call is None:
        die("style 由服务端 bendi_style 计算：请用 occ.py bendi style …（或直接调 MCP 工具 bendi_style）", 2)
    if sleep is None:
        import time
        sleep = time.sleep
    for _ in range(max_polls):
        code, res = call("bendi_style", args)
        if code != 0:
            print(json.dumps(res, ensure_ascii=False, indent=2))
            return code
        if not (isinstance(res, dict) and res.get("status") == "running"):
            break
        sys.stderr.write("[bendi] BGM 节拍还在算（%s），5 秒后重试\n" % res.get("jobId"))
        sleep(5)
    print(json.dumps(res, ensure_ascii=False, indent=2))
    if isinstance(res, dict) and res.get("status") == "running":
        sys.stderr.write("[bendi] 节拍仍未算完：稍后同一条命令再跑\n")
        return 4
    return 1 if isinstance(res, dict) and res.get("pass") is False else 0


def main(argv, call=None):
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    cmd, rest = argv[0], argv[1:]
    if cmd == "state" and rest[:1] == ["check"] and len(rest) == 2:
        fail(check_state(load(rest[1])))
    if cmd == "gate" and rest:
        shot = rest[rest.index("--shot") + 1] if "--shot" in rest else None
        fail(gate_errors(load(rest[0]), shot))
    if cmd == "build-request" and "--shot" in rest:
        st = load(rest[0])
        shot = rest[rest.index("--shot") + 1]
        errs = gate_errors(st, shot)
        req = build_request(st, shot)
        errs += lint_request(req)
        print(json.dumps(req, ensure_ascii=False, indent=2))
        fail(errs)
    if cmd == "lint" and rest:
        fail(lint_request(load(rest[0])))
    if cmd == "budget" and rest:
        price = float(rest[rest.index("--price") + 1]) if "--price" in rest else 0.6
        print(json.dumps(budget(load(rest[0]), price), ensure_ascii=False, indent=2))
        return 0
    if cmd == "crop-plan" and rest:
        print(json.dumps(crop_plan(rest[0]), ensure_ascii=False, indent=2))
        return 0
    if cmd == "to-segments" and rest:
        print(json.dumps(to_segments(load(rest[0])), ensure_ascii=False, indent=2))
        return 0
    if cmd == "style":
        return style(rest, call)
    if cmd == "entry" and rest:
        try:
            print(entry(rest[0], load(rest[1]) if len(rest) > 1 else None))
        except ValueError as e:
            die(str(e), 2)
        return 0
    if cmd == "revise-route" and rest:
        try:
            print(revise_route(rest[0]))
        except ValueError as e:
            die(str(e), 2)
        return 0
    if cmd in ("scope-for", "classify-scope", "hash-file", "fingerprint", "fp-compare", "dedupe", "fp-tags", "check-uploads"):
        if occ_assets is None:
            die(ASSETS_MISSING, 2)
        return occ_assets.main(argv)  # 共用入口：素材命令一律由 shiju-assets 处理
    if cmd == "route" and len(rest) == 2:
        try:
            print(route(load(rest[0]), rest[1]))
        except ValueError as e:
            die(str(e), 2)
        return 0
    if cmd == "next" and len(rest) == 2:
        st = load(rest[0])
        st["next"] = rest[1]
        print(json.dumps(st, ensure_ascii=False, indent=2))
        return 0
    die(__doc__, 2)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]) or 0)
