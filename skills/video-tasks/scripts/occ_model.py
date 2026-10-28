#!/usr/bin/env python3
"""occ_model.py —— cutvideo model 模块（modules/model.md）：AI 达人 / 模特人物的机检工具（只读、不调网关、不花钱、不读数据文件）。

形象简报、提示词拼接和提示词检查在服务端：manage_character {action:"brief" | "fill_prompt" | "lint_prompt"}。
  python3 occ_model.py view-order <character.json>                                          按 character_scene 用的顺序排 views，打印「图k 是…」
  python3 occ_model.py bump-check <old.json> <new.json>                                     改档案前判断要不要 update（只改 note 不升版）
  python3 occ_model.py check-result <result.json>                                           交给 cutvideo 前的结果校验（授权二选一、视图、审图清单）

occ_bendi.py（flows/bendi/steps/step-02a-character.md 的 state check）导入本文件的 check_character()。
"""
import json
import sys

# manage_character 的 skill 角色名（服务端 schema 优先用这些）；旧名映射过来
LEGACY = {"front": "face_front", "three_quarter": "face_34", "side": "face_side", "back": "full_back",
          "full_body": "full_front", "half_body": "half_front", "face": "face_front", "expression": "expr_smile",
          "sheet": "sheet_a"}
LABEL = {"face_front": "正脸近景", "face_34": "3/4 侧脸近景", "face_side": "侧脸", "half_front": "正面坐姿半身",
         "half_34": "3/4 半身", "full_front": "正面全身", "full_34": "3/4 全身", "full_side": "侧面全身",
         "full_back": "背面全身", "expr_talk": "说话表情", "expr_smile": "微笑表情", "expr_taste": "试吃惊喜表情",
         "expr_satisfied": "满足表情", "expr_surprise": "惊讶表情", "other": "补充视角"}
# character_scene 里视图就是 图1…图N：脸放最前（图1 是身份锚点），全身在中间，表情在后；设定页不进档案 views
ORDER = ["face_front", "face_34", "half_front", "face_side", "full_front", "expr_smile", "expr_talk", "expr_taste",
         "expr_satisfied", "expr_surprise", "half_34", "full_34", "full_side", "full_back", "other"]
REQUIRED = ["face_front", "face_34", "half_front", "full_front"]      # 必须（林小满 v5 有这 4 个 + 侧脸 / 表情 / 3/4 半身）
SUGGESTED = ["face_side", "expr_smile", "half_34"]                     # 建议
MAX_VIEWS_FOR_SCENE = 7                                                # 必须：留 ≥3 张给门店环境 / 实物参考（Wan3 共 10 张）
KEY_FIELDS = ["persona", "voiceId", "sampleAudioRef", "views", "outfitId"]   # 服务端：这些变了就自动升版本并清确认
REVIEW_KEYS = {
    "fingersOk": "每只手 5 根手指，指节自然，没有多指 / 融指",
    "faceIntact": "五官不崩：眼睛对称、牙齿正常、没有歪嘴 / 重影",
    "sameIdentityAllViews": "所有视图是同一个人（脸型、五官、发型、配饰位置一致）",
    "outfitConsistent": "所有视图服装、鞋、配饰一致（发夹 / 耳环位置也一致）",
    "noWatermarkBadge": "没有水印、角标、logo、文字",
    "realism": "像手机 / 相机实拍：有毛孔纹理，不是磨皮塑料脸、不是动漫 / CG",
    "beauty": "高颜值、身材好，不是普通长相；穿搭时尚不土（不是牛仔上下装）",
    "notRealPerson": "看不出是哪个真实的人（不像任何明星 / 网红 / 参考达人本人）",
}
DENIM = ["牛仔外套", "牛仔裤", "牛仔套装", "牛仔衬衫", "denim"]
IMG_KINDS = ("character", "character_sheet")


def die(msg, code=1):
    print(msg, file=sys.stderr)
    sys.exit(code)


def _load(path):
    if path == "-":
        return json.load(sys.stdin)
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def norm_role(r):
    return LEGACY.get(r, r)


def view_order(views):
    vs = [dict(v, role=norm_role(v.get("role"))) for v in views if norm_role(v.get("role")) not in ("sheet_a", "sheet_b")]
    rank = {r: i for i, r in enumerate(ORDER)}
    vs.sort(key=lambda v: rank.get(v["role"], len(ORDER)))
    lines = ["图%d 是%s" % (i + 1, v.get("note") or LABEL.get(v["role"], v["role"])) for i, v in enumerate(vs)]
    return vs, lines


def bump_check(old, new):
    """manage_character update 会在 KEY_FIELDS 变化时升版本并清确认。只改 note / 排序 = 不该 update。"""
    changed = []
    for k in KEY_FIELDS:
        a, b = old.get(k), new.get(k)
        if k == "views":
            a = sorted((norm_role(x.get("role")), x.get("assetRef")) for x in (a or []))
            b = sorted((norm_role(x.get("role")), x.get("assetRef")) for x in (b or []))
        if a != b:
            changed.append(k)
    notes_only = not changed and json.dumps(old.get("views"), sort_keys=True) != json.dumps(new.get("views"), sort_keys=True)
    if changed:
        advice = "要 update：%s 变了 → 新版本，之后必须让用户重新确认（confirm）再生成" % "、".join(changed)
    elif notes_only:
        advice = "不要 update：只改了视图 note / 顺序。说明写进工程 state，不动档案（林小满 v4 就是只修 note 白升了一版、清掉了确认）"
    else:
        advice = "不要 update：关键字段没变（rename 不升版本，可以单独做）"
    return {"update": bool(changed), "changed": changed, "notesOnly": notes_only, "advice": advice}


def check_character(ch):
    """cutvideo state.character / model 模块交付结果的共同校验。返回错误列表。"""
    errs = []
    syn, cons = ch.get("synthetic") is True, bool(ch.get("portraitConsentId"))
    if syn == cons:
        errs.append("synthetic:true 和 portraitConsentId 必须二选一（现在 synthetic=%s, portraitConsentId=%s）" % (ch.get("synthetic"), ch.get("portraitConsentId")))
    views = ch.get("views") or []
    roles = [norm_role(v.get("role")) for v in views]
    miss = [r for r in REQUIRED if r not in roles]
    if miss:
        errs.append("缺必须视角：%s" % ",".join(miss))
    for v in views:
        if not (v.get("assetRef") or v.get("assetId")):
            errs.append("视角 %s 没有 assetRef（档案只能按 assetRef 引用，签名 URL 会过期）" % v.get("role"))
    if any(r in ("sheet_a", "sheet_b") for r in roles):
        errs.append("整张设定页不进 views：先裁成单视角（拼图喂视频模型会把多格排版也生成出来）")
    if len(views) > MAX_VIEWS_FOR_SCENE:
        errs.append("views %d 张 > %d：character_scene 会把它们全放在 图1…图N，门店环境 / 实物参考就没位置了" % (len(views), MAX_VIEWS_FOR_SCENE))
    if roles and roles[0] != "face_front":
        errs.append("views 第一张要是正脸（face_front）：character_scene 里它就是 图1 身份锚点（view-order 可以排好）")
    if ch.get("status") == "confirmed" and not ch.get("confirmedBy"):
        errs.append("confirmed 但没有 confirmedBy（记用户原话）")
    return errs


def check_result(res):
    errs = check_character(res)
    warns = []
    if not res.get("characterId") or res.get("version") is None:
        errs.append("缺 characterId / version（manage_character 建档后回填）")
    if res.get("status") != "confirmed":
        errs.append("status=%s：用户没确认的版本不交给 cutvideo" % res.get("status"))
    if not res.get("lookLine"):
        errs.append("缺 lookLine（一句话外形：发型、配饰、服装；cutvideo 的视频提示词照抄它）")
    elif any(w in res["lookLine"] for w in DENIM):
        errs.append("lookLine 里有牛仔上下装")
    rv = res.get("review") or {}
    for k, desc in REVIEW_KEYS.items():
        if rv.get(k) is not True:
            errs.append("审图未通过 / 没勾：%s（%s）" % (k, desc))
    if not (res.get("voice") or {}).get("voiceId"):
        warns.append("没有 voice.voiceId：口播片要在档案里定音色（manage_voice，默认 longanran）")
    if res.get("tier") is None:
        warns.append("没记 tier：换装 / 新建要按探店档位选穿搭")
    return errs, warns


def _out(o):
    print(json.dumps(o, ensure_ascii=False, indent=1))


def main(argv):
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0
    cmd, rest = argv[0], argv[1:]
    if cmd == "view-order" and rest:
        vs, lines = view_order(_load(rest[0]).get("views") or [])
        _out({"views": vs, "lines": lines})
        return 0
    if cmd == "bump-check" and len(rest) >= 2:
        _out(bump_check(_load(rest[0]), _load(rest[1])))
        return 0
    if cmd == "check-result" and rest:
        errs, warns = check_result(_load(rest[0]))
        for w in warns:
            print("△ " + w)
        for e in errs:
            print("✗ " + e)
        print("通过" if not errs else "不通过")
        return 1 if errs else 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
