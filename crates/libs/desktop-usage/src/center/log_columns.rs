//! Stable column keys match the reference. Only deviations from defaults are saved.
use std::{collections::BTreeMap, path::PathBuf};

pub const COLUMNS: [(&str, &str, bool, f32); 14] = [
    ("time", "时间", true, 150.0),
    ("app", "应用", true, 90.0),
    ("provider", "供应商", true, 150.0),
    ("model", "模型", true, 220.0),
    ("cost", "成本", true, 130.0),
    ("freshInput", "新增输入", true, 100.0),
    ("output", "输出", true, 90.0),
    ("cacheRead", "缓存读取", true, 100.0),
    ("cacheWrite", "缓存写入", false, 100.0),
    ("hitRate", "命中率", false, 90.0),
    ("speed", "速度", true, 120.0),
    ("firstToken", "首字", true, 100.0),
    ("duration", "耗时", false, 100.0),
    ("statusCode", "状态码", false, 90.0),
];

#[derive(Default)]
pub struct Preferences {
    overrides: BTreeMap<String, bool>,
}
impl Preferences {
    pub fn visible(&self, index: usize) -> bool {
        COLUMNS.get(index).is_some_and(|(key, _, default, _)| {
            index == 0 || self.overrides.get(*key).copied().unwrap_or(*default)
        })
    }
    pub fn toggle(&mut self, index: usize) {
        if index == 0 {
            return;
        }
        let Some((key, _, default, _)) = COLUMNS.get(index) else {
            return;
        };
        let next = !self.visible(index);
        if next == *default {
            self.overrides.remove(*key);
        } else {
            self.overrides.insert((*key).into(), next);
        }
    }
    fn path() -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or("无法获取用户目录")?;
        Ok(config_runtime_paths::home_config_dir(PathBuf::from(home))
            .join("usage-log-columns.json"))
    }
    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let mut overrides: BTreeMap<String, bool> =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        overrides.retain(|key, value| {
            COLUMNS.iter().skip(1).any(|(id, _, default, _)| key == id && *value != *default)
        });
        Ok(Self { overrides })
    }
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        let parent = path.parent().ok_or("列设置路径无效")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec(&self.overrides).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }
}
