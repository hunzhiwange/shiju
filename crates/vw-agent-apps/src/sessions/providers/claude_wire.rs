//
// 所有字段都是「宽松」类型：类型不符时当作缺省而不是让整行解析失败；
// 字符串尽量借用行缓冲区，图片 base64 等大字段不复制。

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRecord<'a> {
    #[serde(rename = "type", default, borrow)]
    kind: LStr<'a>,
    #[serde(default, borrow)]
    subtype: LStr<'a>,
    #[serde(default)]
    is_meta: Option<Value>,
    #[serde(default)]
    is_sidechain: Option<Value>,
    #[serde(default)]
    is_compact_summary: Option<Value>,
    #[serde(default, borrow)]
    uuid: LStr<'a>,
    #[serde(default)]
    timestamp: Option<Value>,
    #[serde(default, borrow)]
    message: Lenient<RawMessage<'a>>,
    #[serde(default)]
    tool_use_result: Lenient<ToolUseResult>,
    #[serde(default)]
    thinking_duration_ms: Option<Value>,
    #[serde(default)]
    hook_errors: Option<Value>,
    /// system 记录的正文（compact_boundary / local_command）
    #[serde(default, borrow)]
    content: LStr<'a>,
    #[serde(default, borrow)]
    pr_url: LStr<'a>,
    #[serde(default)]
    pr_number: Option<Value>,
}

#[derive(Deserialize)]
struct RawMessage<'a> {
    #[serde(default, borrow)]
    role: LStr<'a>,
    #[serde(default, borrow)]
    id: LStr<'a>,
    #[serde(default, borrow)]
    model: LStr<'a>,
    #[serde(default, borrow)]
    stop_reason: LStr<'a>,
    #[serde(default)]
    usage: Option<Value>,
    #[serde(default, borrow)]
    content: RawContent<'a>,
}

/// `message.content[]` 与 `tool_result.content[]` 的元素（字段取并集）
#[derive(Deserialize)]
struct RawItem<'a> {
    #[serde(rename = "type", default, borrow)]
    kind: LStr<'a>,
    #[serde(default, borrow)]
    text: LStr<'a>,
    #[serde(default, borrow)]
    thinking: LStr<'a>,
    #[serde(default, borrow)]
    signature: LStr<'a>,
    #[serde(default, borrow)]
    id: LStr<'a>,
    #[serde(default, borrow)]
    name: LStr<'a>,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default, borrow)]
    tool_use_id: LStr<'a>,
    #[serde(default)]
    is_error: Option<Value>,
    #[serde(default, borrow)]
    content: RawContent<'a>,
    #[serde(default, borrow)]
    source: Lenient<RawSource<'a>>,
}

#[derive(Deserialize)]
struct RawSource<'a> {
    #[serde(rename = "type", default, borrow)]
    kind: LStr<'a>,
    #[serde(default, borrow)]
    media_type: LStr<'a>,
    #[serde(default, borrow)]
    data: LStr<'a>,
}

/// 顶层 `toolUseResult`：只取回填需要的字段（stdout、file、originalFile 等大字段跳过）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolUseResult {
    #[serde(default)]
    interrupted: Option<Value>,
    #[serde(default)]
    persisted_output_path: Option<Value>,
    #[serde(rename = "type", default)]
    kind: Option<Value>,
    #[serde(default)]
    file_path: Option<Value>,
    #[serde(default)]
    resolved_model: Option<Value>,
    #[serde(default)]
    background_task_id: Option<Value>,
    #[serde(default)]
    structured_patch: Option<PatchCounts>,
}

/// 宽松的借用字符串：非字符串值当作 `None`。
#[derive(Default)]
struct LStr<'a>(Option<Cow<'a, str>>);

impl LStr<'_> {
    fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// 只接受对象的宽松包装：其他类型当作 `None`。
struct Lenient<T>(Option<T>);

impl<T> Default for Lenient<T> {
    fn default() -> Self {
        Self(None)
    }
}

/// 字符串或块数组（`message.content` / `tool_result.content`）
#[derive(Default)]
enum RawContent<'a> {
    #[default]
    None,
    Text(Cow<'a, str>),
    Items(Vec<Lenient<RawItem<'a>>>),
}

/// `structuredPatch` 里以 `+` / `-` 开头的行数
#[derive(Clone, Copy, Default)]
struct PatchCounts {
    added: u32,
    removed: u32,
}

/// 非预期类型的值整段跳过，返回 `$value`。
macro_rules! skip_other_values {
    ($value:expr) => {
        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($value)
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok($value)
        }
    };
}

/// 字符串值整段跳过，返回 `$value`。
macro_rules! skip_str_values {
    ($value:expr) => {
        fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
            Ok($value)
        }
    };
}

/// 数组 / 对象整段跳过，返回 `$value`。
macro_rules! skip_seq_values {
    ($value:expr) => {
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            while seq.next_element::<IgnoredAny>()?.is_some() {}
            Ok($value)
        }
    };
}

macro_rules! skip_map_values {
    ($value:expr) => {
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok($value)
        }
    };
}

impl<'de: 'a, 'a> Deserialize<'de> for LStr<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<'a>(PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for V<'a> {
            type Value = LStr<'a>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any value")
            }
            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(LStr(Some(Cow::Borrowed(v))))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(LStr(Some(Cow::Owned(v.to_string()))))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(LStr(Some(Cow::Owned(v))))
            }
            skip_other_values!(LStr(None));
            skip_seq_values!(LStr(None));
            skip_map_values!(LStr(None));
        }
        deserializer.deserialize_any(V(PhantomData))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Lenient<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Lenient<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any value")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(|v| Lenient(Some(v)))
            }
            skip_other_values!(Lenient(None));
            skip_str_values!(Lenient(None));
            skip_seq_values!(Lenient(None));
        }
        deserializer.deserialize_any(V(PhantomData))
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawContent<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<'a>(PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for V<'a> {
            type Value = RawContent<'a>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("string or array")
            }
            fn visit_borrowed_str<E: de::Error>(self, v: &'de str) -> Result<Self::Value, E> {
                Ok(RawContent::Text(Cow::Borrowed(v)))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(RawContent::Text(Cow::Owned(v.to_string())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(RawContent::Text(Cow::Owned(v)))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(item) = seq.next_element::<Lenient<RawItem<'a>>>()? {
                    items.push(item);
                }
                Ok(RawContent::Items(items))
            }
            skip_other_values!(RawContent::None);
            skip_map_values!(RawContent::None);
        }
        deserializer.deserialize_any(V(PhantomData))
    }
}

impl<'de> Deserialize<'de> for PatchCounts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// hunk 对象：只看 `lines`
        struct Hunk;
        impl<'de> Visitor<'de> for Hunk {
            type Value = PatchCounts;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("hunk")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut counts = PatchCounts::default();
                while let Some(key) = map.next_key::<LStr<'_>>()? {
                    if key.as_str() == Some("lines") {
                        counts = map.next_value_seed(Lines)?;
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(counts)
            }
            skip_other_values!(PatchCounts::default());
            skip_str_values!(PatchCounts::default());
            skip_seq_values!(PatchCounts::default());
        }

        /// `lines` 数组：按首字符计数
        struct Lines;
        impl<'de> de::DeserializeSeed<'de> for Lines {
            type Value = PatchCounts;
            fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(self)
            }
        }
        impl<'de> Visitor<'de> for Lines {
            type Value = PatchCounts;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("lines")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut counts = PatchCounts::default();
                while let Some(line) = seq.next_element::<LStr<'_>>()? {
                    match line.as_str().and_then(|l| l.chars().next()) {
                        Some('+') => counts.added += 1,
                        Some('-') => counts.removed += 1,
                        _ => {}
                    }
                }
                Ok(counts)
            }
            skip_other_values!(PatchCounts::default());
            skip_str_values!(PatchCounts::default());
            skip_map_values!(PatchCounts::default());
        }

        /// 外层 hunk 数组：累加
        struct Hunks;
        impl<'de> Visitor<'de> for Hunks {
            type Value = PatchCounts;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("structuredPatch")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut total = PatchCounts::default();
                while let Some(hunk) = seq.next_element_seed(HunkSeed)? {
                    total.added += hunk.added;
                    total.removed += hunk.removed;
                }
                Ok(total)
            }
            skip_other_values!(PatchCounts::default());
            skip_str_values!(PatchCounts::default());
            skip_map_values!(PatchCounts::default());
        }
        struct HunkSeed;
        impl<'de> de::DeserializeSeed<'de> for HunkSeed {
            type Value = PatchCounts;
            fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(Hunk)
            }
        }

        deserializer.deserialize_any(Hunks)
    }
}

