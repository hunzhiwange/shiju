//! 定义技能市场、安装记录、版本详情与下载地址等传输模型。

use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 技能列表用于筛选和展示的标签。
pub(crate) struct SkillTag {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 技能市场列表中的可安装技能摘要。
pub(crate) struct MarketplaceSkill {
    pub(crate) id: u64,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) level: String,
    pub(crate) modality: String,
    pub(crate) format: String,
    pub(crate) company_id: String,
    pub(crate) created_uid: String,
    pub(crate) maintainer_uid: String,
    pub(crate) status: String,
    pub(crate) is_all_company_accessible: u8,
    pub(crate) current_version_id: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) deleted_at: Option<i64>,
    pub(crate) is_installed: bool,
    pub(crate) tags: Vec<SkillTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 当前账号已安装技能的版本摘要。
pub(crate) struct InstalledSkill {
    pub(crate) skill_id: u64,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) installed_version: String,
    pub(crate) current_version: String,
    pub(crate) format: String,
    pub(crate) upgrade_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 系统提供的技能摘要。
pub(crate) struct SystemSkill {
    pub(crate) id: u64,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) is_all_company_accessible: u8,
    pub(crate) tags: Vec<SkillTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 公司或当前用户维护的技能摘要及审核状态。
pub(crate) struct TenantSkill {
    pub(crate) id: u64,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) is_all_company_accessible: u8,
    pub(crate) has_pending_review: bool,
    pub(crate) pending_review_version_id: Option<u64>,
    pub(crate) tags: Vec<SkillTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 使用页码和总数表示的技能列表页。
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) page: u32,
    pub(crate) page_size: u32,
    pub(crate) total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 使用不透明游标继续加载的技能列表页。
pub(crate) struct CursorPage<T> {
    pub(crate) items: Vec<T>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
}

impl From<vw_gateway_client::ShijuSkillTagDto> for SkillTag {
    fn from(dto: vw_gateway_client::ShijuSkillTagDto) -> Self {
        Self { id: dto.id, name: dto.name, sort_order: dto.sort_order }
    }
}

impl From<vw_gateway_client::ShijuMarketplaceSkillDto> for MarketplaceSkill {
    fn from(dto: vw_gateway_client::ShijuMarketplaceSkillDto) -> Self {
        Self {
            id: dto.id,
            slug: dto.slug,
            name: dto.name,
            description: dto.description,
            level: dto.level,
            modality: dto.modality,
            format: dto.format,
            company_id: dto.company_id,
            created_uid: dto.created_uid,
            maintainer_uid: dto.maintainer_uid,
            status: dto.status,
            is_all_company_accessible: dto.is_all_company_accessible,
            current_version_id: dto.current_version_id,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
            deleted_at: dto.deleted_at,
            is_installed: dto.is_installed,
            tags: dto.tags.into_iter().map(SkillTag::from).collect(),
        }
    }
}

impl From<vw_gateway_client::ShijuSkillCursorPageDto> for CursorPage<MarketplaceSkill> {
    fn from(dto: vw_gateway_client::ShijuSkillCursorPageDto) -> Self {
        Self {
            items: dto.items.into_iter().map(MarketplaceSkill::from).collect(),
            next_cursor: dto.next_cursor,
            has_more: dto.has_more,
        }
    }
}

impl From<vw_gateway_client::ShijuInstalledSkillDto> for InstalledSkill {
    fn from(dto: vw_gateway_client::ShijuInstalledSkillDto) -> Self {
        Self {
            skill_id: dto.skill_id,
            slug: dto.slug,
            name: dto.name,
            description: dto.description,
            installed_version: dto.installed_version,
            current_version: dto.current_version,
            format: dto.format,
            upgrade_available: dto.upgrade_available,
        }
    }
}

impl From<vw_gateway_client::ShijuSystemSkillDto> for SystemSkill {
    fn from(dto: vw_gateway_client::ShijuSystemSkillDto) -> Self {
        Self {
            id: dto.id,
            slug: dto.slug,
            name: dto.name,
            description: dto.description,
            is_all_company_accessible: dto.is_all_company_accessible,
            tags: dto.tags.into_iter().map(SkillTag::from).collect(),
        }
    }
}

impl From<vw_gateway_client::ShijuTenantSkillDto> for TenantSkill {
    fn from(dto: vw_gateway_client::ShijuTenantSkillDto) -> Self {
        Self {
            id: dto.id,
            slug: dto.slug,
            name: dto.name,
            description: dto.description,
            status: dto.status,
            is_all_company_accessible: dto.is_all_company_accessible,
            has_pending_review: dto.has_pending_review,
            pending_review_version_id: dto.pending_review_version_id,
            tags: dto.tags.into_iter().map(SkillTag::from).collect(),
        }
    }
}

impl From<vw_gateway_client::ShijuSkillPageDto<vw_gateway_client::ShijuSystemSkillDto>>
    for Page<SystemSkill>
{
    fn from(
        dto: vw_gateway_client::ShijuSkillPageDto<vw_gateway_client::ShijuSystemSkillDto>,
    ) -> Self {
        Self {
            items: dto.items.into_iter().map(SystemSkill::from).collect(),
            page: dto.page,
            page_size: dto.page_size,
            total: dto.total,
        }
    }
}

impl From<vw_gateway_client::ShijuSkillPageDto<vw_gateway_client::ShijuTenantSkillDto>>
    for Page<TenantSkill>
{
    fn from(
        dto: vw_gateway_client::ShijuSkillPageDto<vw_gateway_client::ShijuTenantSkillDto>,
    ) -> Self {
        Self {
            items: dto.items.into_iter().map(TenantSkill::from).collect(),
            page: dto.page,
            page_size: dto.page_size,
            total: dto.total,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 技能详情及当前版本内容。
pub(crate) struct SkillDetail {
    pub(crate) id: u64,
    pub(crate) slug: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) level: String,
    pub(crate) modality: String,
    pub(crate) format: String,
    pub(crate) company_id: String,
    pub(crate) created_uid: String,
    pub(crate) maintainer_uid: String,
    pub(crate) status: String,
    pub(crate) is_all_company_accessible: u8,
    pub(crate) current_version_id: u64,
    pub(crate) current_version: Option<SkillVersionContent>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) tags: Vec<SkillTag>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 已校验下载地址的技能版本内容。
pub(crate) struct SkillVersionContent {
    pub(crate) checksum: String,
    pub(crate) content_type: String,
    pub(crate) download_expires_at: i64,
    pub(crate) download_url: SkillDownloadUrl,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 可供升级、回滚或下载的技能版本记录。
pub(crate) struct SkillVersion {
    pub(crate) id: u64,
    pub(crate) skill_id: u64,
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) change_type: String,
    #[serde(default)]
    pub(crate) changelog: Option<String>,
    #[serde(default)]
    pub(crate) checksum: String,
    #[serde(default)]
    pub(crate) content_type: String,
    #[serde(default)]
    pub(crate) download_expires_at: i64,
    #[serde(default, deserialize_with = "deserialize_optional_skill_download_url")]
    pub(crate) download_url: Option<SkillDownloadUrl>,
    #[serde(default)]
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) updated_at: String,
}

impl TryFrom<vw_gateway_client::ShijuSkillVersionContentDto> for SkillVersionContent {
    type Error = String;

    fn try_from(dto: vw_gateway_client::ShijuSkillVersionContentDto) -> Result<Self, Self::Error> {
        let download_url = dto
            .download_url
            .ok_or_else(|| "技能下载地址不可用".to_string())
            .and_then(|url| SkillDownloadUrl::parse(&url))?;
        Ok(Self {
            checksum: dto.checksum,
            content_type: dto.content_type,
            download_expires_at: dto.download_expires_at,
            download_url,
            version: dto.version,
        })
    }
}

impl TryFrom<vw_gateway_client::ShijuSkillDetailDto> for SkillDetail {
    type Error = String;

    fn try_from(dto: vw_gateway_client::ShijuSkillDetailDto) -> Result<Self, Self::Error> {
        Ok(Self {
            id: dto.id,
            slug: dto.slug,
            name: dto.name,
            description: dto.description,
            level: dto.level,
            modality: dto.modality,
            format: dto.format,
            company_id: dto.company_id,
            created_uid: dto.created_uid,
            maintainer_uid: dto.maintainer_uid,
            status: dto.status,
            is_all_company_accessible: dto.is_all_company_accessible,
            current_version_id: dto.current_version_id,
            current_version: dto.current_version.map(SkillVersionContent::try_from).transpose()?,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
            tags: dto.tags.into_iter().map(SkillTag::from).collect(),
        })
    }
}

impl From<vw_gateway_client::ShijuSkillVersionDto> for SkillVersion {
    fn from(dto: vw_gateway_client::ShijuSkillVersionDto) -> Self {
        Self {
            id: dto.id,
            skill_id: dto.skill_id,
            version: dto.version,
            status: dto.status,
            change_type: dto.change_type,
            changelog: dto.changelog,
            checksum: dto.checksum,
            content_type: dto.content_type,
            download_expires_at: dto.download_expires_at,
            download_url: dto.download_url.and_then(|url| SkillDownloadUrl::parse(&url).ok()),
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }
}

fn deserialize_optional_skill_download_url<'de, D>(
    deserializer: D,
) -> Result<Option<SkillDownloadUrl>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|value| SkillDownloadUrl::parse(&value).ok()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 识矩安装包支持的操作系统与处理器组合。
pub(crate) enum ShijuPackageId {
    DarwinAmd64,
    DarwinArm64,
    LinuxAmd64,
    WindowsAmd64,
}

impl ShijuPackageId {
    pub(crate) const ALL: [Self; 4] =
        [Self::DarwinAmd64, Self::DarwinArm64, Self::LinuxAmd64, Self::WindowsAmd64];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::DarwinAmd64 => "shiju-darwin-amd64",
            Self::DarwinArm64 => "shiju-darwin-arm64",
            Self::LinuxAmd64 => "shiju-linux-amd64",
            Self::WindowsAmd64 => "shiju-windows-amd64",
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::DarwinAmd64 => "macOS Intel",
            Self::DarwinArm64 => "macOS Apple Silicon",
            Self::LinuxAmd64 => "Linux amd64",
            Self::WindowsAmd64 => "Windows amd64",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::DarwinAmd64 => "适用于 Intel 芯片 Mac。",
            Self::DarwinArm64 => "适用于 M 系列芯片 Mac。",
            Self::LinuxAmd64 => "适用于 x86_64 Linux。",
            Self::WindowsAmd64 => "适用于 64 位 Windows。",
        }
    }

    /// 返回当前编译目标对应的安装包，不受支持的平台返回 `None`。
    pub(crate) fn current() -> Option<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "x86_64") => Some(Self::DarwinAmd64),
            ("macos", "aarch64") => Some(Self::DarwinArm64),
            ("linux", "x86_64") => Some(Self::LinuxAmd64),
            ("windows", "x86_64") => Some(Self::WindowsAmd64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
/// 技能中心返回的识矩包发布信息。
pub(crate) struct ShijuPackageRelease {
    pub(crate) download_url: SkillDownloadUrl,
}

impl ShijuPackageRelease {
    /// 从受信任下载路径的 `releases/{version}` 段提取纯数字版本。
    pub(crate) fn version(&self) -> Option<&str> {
        let segments = self.download_url.0.path_segments()?.collect::<Vec<_>>();
        let version =
            segments.windows(2).find_map(|pair| (pair[0] == "releases").then_some(pair[1]))?;
        is_numeric_version(version).then_some(version)
    }
}

fn is_numeric_version(version: &str) -> bool {
    !version.is_empty()
        && version.split('.').all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[derive(Clone, PartialEq, Eq)]
/// 仅允许识矩技能下载主机的 HTTPS 地址；调试输出会脱敏。
pub(crate) struct SkillDownloadUrl(reqwest::Url);

impl SkillDownloadUrl {
    /// 校验协议、主机、端口和用户信息后构造下载地址。
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let url = reqwest::Url::parse(value).map_err(|_| "技能下载地址格式无效".to_string())?;
        if url.scheme() != "https"
            || url.host_str() != Some("skill.shiju.cloud")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(443)
        {
            return Err("技能下载地址不受信任".to_string());
        }
        Ok(Self(url))
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for SkillDownloadUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SkillDownloadUrl(<redacted>)")
    }
}

impl<'de> Deserialize<'de> for SkillDownloadUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}
