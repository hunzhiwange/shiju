//! 公共 SkillHub 目录的技能市场适配器。

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::future::try_join_all;
use reqwest::header::LOCATION;
use reqwest::redirect::Policy;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer};
use serde_json::Value;
use url::Url;
use vw_gateway_client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketSecurityDto,
    SkillMarketSkillDetailDto, SkillMarketSkillDto, SkillMarketSkillFileDto,
    SkillMarketSkillLocatorDto, SkillMarketSkillPageDto, SkillMarketSkillQuery,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto, SkillMarketSkillSetPageDto,
    SkillMarketSort, SkillMarketSourceDto,
};

use super::provider::{
    SkillMarketArtifact, SkillMarketArtifactProof, SkillMarketProvider, SkillMarketProviderError,
    SkillMarketSkillSetArtifact, SkillMarketSkillSetMemberArtifact,
    SkillMarketSkillSetVerificationPolicy,
};

const SOURCE: &str = "skillhub";
const DISPLAY_NAME: &str = "SkillHub";
const PRODUCTION_BASE_URL: &str = "https://api.skillhub.cn";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const SKILL_FILE_PATH: &str = "SKILL.md";
const SKILL_FILE_REDIRECT_HOST: &str = "skillhub-1388575217.cos.accelerate.myqcloud.com";
const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;

/// 公共 SkillHub HTTP API 的市场提供方实现。
pub(crate) struct SkillHubProvider {
    source: SkillMarketSourceDto,
    client: Client,
    base_url: Url,
}

impl SkillHubProvider {
    /// 使用固定生产端点创建提供方。
    pub(crate) fn new() -> Result<Self, SkillMarketProviderError> {
        Self::build(PRODUCTION_BASE_URL)
    }

    #[cfg(test)]
    /// 使用显式基础地址创建提供方，供受控环境与测试使用。
    pub(crate) fn with_base_url(base_url: &str) -> Result<Self, SkillMarketProviderError> {
        Self::build(base_url)
    }

    fn build(base_url: &str) -> Result<Self, SkillMarketProviderError> {
        let client =
            Client::builder().timeout(REQUEST_TIMEOUT).redirect(Policy::none()).build().map_err(
                |_| {
                    SkillMarketProviderError::InvalidResponse(
                        "failed to build the SkillHub HTTP client".to_string(),
                    )
                },
            )?;
        let base_url = Url::parse(base_url).map_err(|_| {
            SkillMarketProviderError::InvalidResponse("SkillHub base URL is invalid".to_string())
        })?;
        if !matches!(base_url.scheme(), "http" | "https") || base_url.cannot_be_a_base() {
            return Err(SkillMarketProviderError::InvalidResponse(
                "SkillHub base URL is invalid".to_string(),
            ));
        }

        Ok(Self {
            source: SkillMarketSourceDto {
                source: SOURCE.to_string(),
                display_name: DISPLAY_NAME.to_string(),
            },
            client,
            base_url,
        })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url, SkillMarketProviderError> {
        let mut url = self.base_url.clone();
        url.set_query(None);
        url.set_fragment(None);
        let mut path = url.path_segments_mut().map_err(|_| {
            SkillMarketProviderError::InvalidResponse(
                "SkillHub base URL cannot contain path segments".to_string(),
            )
        })?;
        path.clear();
        path.extend(segments);
        drop(path);
        Ok(url)
    }

    async fn get_json<T: DeserializeOwned>(&self, url: Url) -> Result<T, SkillMarketProviderError> {
        let response = self.client.get(url).send().await.map_err(map_request_error)?;
        ensure_success(response.status())?;
        response.json::<T>().await.map_err(|_| {
            SkillMarketProviderError::InvalidResponse("SkillHub returned invalid JSON".to_string())
        })
    }

    async fn get_text(&self, url: Url) -> Result<String, SkillMarketProviderError> {
        let response = self.client.get(url).send().await.map_err(map_request_error)?;
        let response = if response.status().is_redirection() {
            let location =
                response.headers().get(LOCATION).and_then(|value| value.to_str().ok()).ok_or_else(
                    || {
                        SkillMarketProviderError::InvalidResponse(
                            "SkillHub file redirect is missing a valid location".to_string(),
                        )
                    },
                )?;
            let redirect_url = skill_file_redirect_url(location)?;
            self.client.get(redirect_url).send().await.map_err(map_request_error)?
        } else {
            response
        };
        ensure_success(response.status())?;
        response.text().await.map_err(|_| {
            SkillMarketProviderError::InvalidResponse(
                "SkillHub returned invalid text content".to_string(),
            )
        })
    }

    async fn get_artifact_bytes(&self, url: Url) -> Result<Vec<u8>, SkillMarketProviderError> {
        let response = self.client.get(url).send().await.map_err(map_request_error)?;
        let mut response = if response.status().is_redirection() {
            let location =
                response.headers().get(LOCATION).and_then(|value| value.to_str().ok()).ok_or_else(
                    || {
                        SkillMarketProviderError::InvalidResponse(
                            "SkillHub artifact redirect is missing a valid location".to_string(),
                        )
                    },
                )?;
            let redirect_url = skill_artifact_redirect_url(location)?;
            self.client.get(redirect_url).send().await.map_err(map_request_error)?
        } else {
            response
        };
        ensure_success(response.status())?;
        if response.content_length().is_some_and(|length| length > MAX_ARTIFACT_BYTES as u64) {
            return Err(SkillMarketProviderError::InvalidResponse(
                "SkillHub artifact exceeds the download limit".to_string(),
            ));
        }

        let mut artifact = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(map_request_error)? {
            let next_len = artifact.len().checked_add(chunk.len()).ok_or_else(|| {
                SkillMarketProviderError::InvalidResponse(
                    "SkillHub artifact size overflowed".to_string(),
                )
            })?;
            if next_len > MAX_ARTIFACT_BYTES {
                return Err(SkillMarketProviderError::InvalidResponse(
                    "SkillHub artifact exceeds the download limit".to_string(),
                ));
            }
            artifact.extend_from_slice(&chunk);
        }
        Ok(artifact)
    }

    fn ensure_source(&self, source: &str) -> Result<(), SkillMarketProviderError> {
        if source == SOURCE {
            Ok(())
        } else {
            Err(SkillMarketProviderError::InvalidResponse(
                "skill coordinate does not belong to SkillHub".to_string(),
            ))
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl SkillMarketProvider for SkillHubProvider {
    fn source(&self) -> &SkillMarketSourceDto {
        &self.source
    }

    async fn recommended(&self) -> Result<Vec<SkillMarketSkillDto>, SkillMarketProviderError> {
        let url = self.endpoint(&["api", "v1", "showcase", "recommended"])?;
        let response: RawRecommended = self.get_json(url).await?;
        response.skills.into_iter().map(map_skill).collect()
    }

    async fn skills(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, SkillMarketProviderError> {
        let mut url = self.endpoint(&["api", "skills"])?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("page", &query.page.to_string());
            pairs.append_pair("pageSize", &query.page_size.to_string());
            pairs.append_pair("sortBy", sort_value(query.sort));
            pairs.append_pair("order", "desc");
            if let Some(keyword) = non_empty(query.keyword.as_deref()) {
                pairs.append_pair("keyword", keyword);
            }
            if let Some(category) = non_empty(query.category.as_deref()) {
                pairs.append_pair("category", category);
            }
            if let Some(namespace) = non_empty(query.namespace.as_deref()) {
                pairs.append_pair("namespace", namespace);
            }
        }
        let response: RawEnvelope<RawSkillPage> = self.get_json(url).await?;
        let data = response.into_data()?;
        Ok(SkillMarketSkillPageDto {
            skills: data.skills.into_iter().map(map_skill).collect::<Result<_, _>>()?,
            total: data.total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn categories(&self) -> Result<Vec<SkillMarketCategoryDto>, SkillMarketProviderError> {
        let url = self.endpoint(&["api", "v1", "categories"])?;
        let response: RawCategories = self.get_json(url).await?;
        try_join_all(response.items.into_iter().map(|category| async move {
            let skill_count = match category.skill_count {
                Some(skill_count) => skill_count,
                None => {
                    self.skills(&SkillMarketSkillQuery {
                        category: Some(category.key.clone()),
                        page_size: 1,
                        ..SkillMarketSkillQuery::default()
                    })
                    .await?
                    .total
                }
            };
            Ok::<_, SkillMarketProviderError>(SkillMarketCategoryDto {
                slug: category.key,
                name: category.name,
                skill_count,
            })
        }))
        .await
    }

    async fn skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, SkillMarketProviderError> {
        self.ensure_source(&locator.source)?;
        let mut url = self.endpoint(&["api", "v1", "skills", &locator.slug])?;
        url.query_pairs_mut().append_pair("namespace", &locator.namespace);
        let response: RawSkillDetail = self.get_json(url).await?;
        map_skill_detail(response)
    }

    async fn skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, SkillMarketProviderError> {
        let detail = self.skill_detail(locator).await?;
        let mut url = self.endpoint(&["api", "v1", "skills", &locator.slug, "file"])?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("path", SKILL_FILE_PATH);
            pairs.append_pair("version", &detail.skill.coordinate.version);
            pairs.append_pair("namespace", &locator.namespace);
        }
        let content = self.get_text(url).await?;
        Ok(SkillMarketSkillFileDto { coordinate: detail.skill.coordinate, content })
    }

    async fn skill_artifact(
        &self,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<SkillMarketArtifact, SkillMarketProviderError> {
        self.ensure_source(&coordinate.locator.source)?;
        let namespace = required_value(&coordinate.locator.namespace, "namespace")?;
        let slug = required_value(&coordinate.locator.slug, "slug")?;
        let version = required_value(&coordinate.version, "version")?;

        let mut download_url = self.endpoint(&["api", "v1", "download"])?;
        download_url
            .query_pairs_mut()
            .append_pair("slug", slug)
            .append_pair("version", version)
            .append_pair("namespace", namespace);
        let archive = self.get_artifact_bytes(download_url).await?;

        let mut signature_url = self.endpoint(&[
            "api",
            "v1",
            "open",
            "skills",
            slug,
            "versions",
            version,
            "signature",
        ])?;
        signature_url.query_pairs_mut().append_pair("namespace", namespace);
        let signature: RawArtifactSignature = self.get_json(signature_url).await?;
        let (key_id, payload, signature_b64, public_key_b64) = if signature.signed {
            let key_id = required(signature.key_id, "signature key ID")?;
            let payload = required(signature.payload, "signature payload")?;
            let signature_b64 = required(signature.signature, "artifact signature")?;
            let keys_url = self.endpoint(&["api", "v1", "open", "platform", "keys"])?;
            let keys: RawPlatformKeys = self.get_json(keys_url).await?;
            let key = keys.keys.into_iter().find(|key| key.key_id == key_id).ok_or_else(|| {
                SkillMarketProviderError::InvalidResponse(
                    "SkillHub signature key is unavailable".to_string(),
                )
            })?;
            if key.algorithm != "Ed25519" {
                return Err(SkillMarketProviderError::InvalidResponse(
                    "SkillHub signature key algorithm is unsupported".to_string(),
                ));
            }
            (key_id, payload, signature_b64, key.public_key_raw_b64)
        } else {
            (String::new(), String::new(), String::new(), String::new())
        };

        Ok(SkillMarketArtifact {
            archive,
            proof: SkillMarketArtifactProof {
                signed: signature.signed,
                payload,
                signature_b64,
                content_hash: signature.content_hash,
                key_id,
                public_key_b64,
            },
        })
    }

    async fn skill_sets(
        &self,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, SkillMarketProviderError> {
        let mut url = self.endpoint(&["api", "v1", "skillsets"])?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("page", &query.page.to_string());
            pairs.append_pair("pageSize", &query.page_size.to_string());
            if let Some(keyword) = non_empty(query.keyword.as_deref()) {
                pairs.append_pair("keyword", keyword);
            }
            if let Some(category) = non_empty(query.category.as_deref()) {
                pairs.append_pair("scene", category);
            }
        }
        let response: RawSkillSetPage = self.get_json(url).await?;
        Ok(SkillMarketSkillSetPageDto {
            skillsets: response.skill_sets.into_iter().map(map_skill_set).collect(),
            total: response.total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    async fn skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, SkillMarketProviderError> {
        self.ensure_source(source)?;
        let url = self.endpoint(&["api", "v1", "skillsets", slug])?;
        let response: RawSkillSet = self.get_json(url).await?;
        let members = response
            .skills
            .iter()
            .map(|member| SkillMarketCoordinateDto {
                locator: SkillMarketSkillLocatorDto {
                    source: SOURCE.to_string(),
                    namespace: member.namespace.clone(),
                    slug: member.slug.clone(),
                },
                version: member.version.clone().unwrap_or_default(),
            })
            .collect();
        let content = response.content.clone().unwrap_or_default();
        Ok(SkillMarketSkillSetDetailDto { skill_set: map_skill_set(response), members, content })
    }

    async fn skill_set_artifact(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetArtifact, SkillMarketProviderError> {
        self.ensure_source(source)?;
        let slug = required_value(slug, "skill-set slug")?;
        let detail_url = self.endpoint(&["api", "v1", "skillsets", slug])?;
        let detail: RawSkillSet = self.get_json(detail_url).await?;
        if detail.slug.as_deref() != Some(slug) {
            return Err(SkillMarketProviderError::InvalidResponse(
                "SkillHub skill-set detail does not match the requested slug".to_string(),
            ));
        }

        let mut coordinates = Vec::with_capacity(detail.skills.len());
        for (index, member) in detail.skills.into_iter().enumerate() {
            let member_name = if member.slug.trim().is_empty() {
                format!("#{}", index + 1)
            } else {
                member.slug.clone()
            };
            let missing = |field| {
                SkillMarketProviderError::InvalidResponse(format!(
                    "SkillHub skill-set member {member_name} is missing {field}"
                ))
            };
            let locator = SkillMarketSkillLocatorDto {
                source: SOURCE.to_string(),
                namespace: required_value(&member.namespace, "namespace")
                    .map_err(|_| missing("namespace"))?
                    .to_string(),
                slug: required_value(&member.slug, "slug")
                    .map_err(|_| missing("slug"))?
                    .to_string(),
            };
            let coordinate = if let Some(version) =
                member.version.filter(|version| !version.trim().is_empty())
            {
                SkillMarketCoordinateDto { locator, version }
            } else {
                let resolved = self.skill_detail(&locator).await?.skill.coordinate;
                if resolved.locator != locator {
                    return Err(SkillMarketProviderError::InvalidResponse(format!(
                        "SkillHub skill-set member {member_name} detail does not match its namespace and slug"
                    )));
                }
                required_value(&resolved.version, "version").map_err(|_| missing("version"))?;
                resolved
            };
            coordinates.push(coordinate);
        }

        let download_url = self.endpoint(&["api", "v1", "skillsets", slug, "download"])?;
        let archive = self.get_artifact_bytes(download_url).await?;
        let mut members = Vec::with_capacity(coordinates.len());
        for coordinate in coordinates {
            let member_artifact = self.skill_artifact(&coordinate).await?;
            members
                .push(SkillMarketSkillSetMemberArtifact { coordinate, artifact: member_artifact });
        }

        Ok(SkillMarketSkillSetArtifact {
            source: SOURCE.to_string(),
            slug: slug.to_string(),
            verification: SkillMarketSkillSetVerificationPolicy::TransportAndAudit,
            archive,
            members,
        })
    }
}

/// 校验技能文件重定向地址，仅接受允许主机上的 HTTPS 下载路径。
pub(super) fn skill_file_redirect_url(location: &str) -> Result<Url, SkillMarketProviderError> {
    let url = Url::parse(location).map_err(|_| {
        SkillMarketProviderError::InvalidResponse(
            "SkillHub file redirect URL is invalid".to_string(),
        )
    })?;
    let trusted_path = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .is_some_and(|segments| match segments.as_slice() {
            ["skills", slug, version, "files", SKILL_FILE_PATH] => {
                !slug.is_empty() && !version.is_empty()
            }
            ["orgs", organization, slug, version, "files", SKILL_FILE_PATH] => {
                !organization.is_empty() && !slug.is_empty() && !version.is_empty()
            }
            _ => false,
        });
    if url.scheme() != "https"
        || url.host_str() != Some(SKILL_FILE_REDIRECT_HOST)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !trusted_path
    {
        return Err(SkillMarketProviderError::InvalidResponse(
            "SkillHub file redirect target is not trusted".to_string(),
        ));
    }
    Ok(url)
}

/// 校验技能制品重定向地址及 ZIP 归档路径。
pub(super) fn skill_artifact_redirect_url(location: &str) -> Result<Url, SkillMarketProviderError> {
    let url = Url::parse(location).map_err(|_| {
        SkillMarketProviderError::InvalidResponse(
            "SkillHub artifact redirect URL is invalid".to_string(),
        )
    })?;
    let trusted_path = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .is_some_and(|segments| match segments.as_slice() {
            ["skills", slug, archive] => !slug.is_empty() && valid_zip_archive(archive),
            ["skills", skill_id, slug, archive] => {
                skill_id.bytes().all(|byte| byte.is_ascii_digit())
                    && !skill_id.is_empty()
                    && !slug.is_empty()
                    && valid_zip_archive(archive)
            }
            _ => false,
        });
    if url.scheme() != "https"
        || url.host_str() != Some(SKILL_FILE_REDIRECT_HOST)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !trusted_path
    {
        return Err(SkillMarketProviderError::InvalidResponse(
            "SkillHub artifact redirect target is not trusted".to_string(),
        ));
    }
    Ok(url)
}

fn valid_zip_archive(archive: &str) -> bool {
    archive.strip_suffix(".zip").is_some_and(|version| !version.is_empty())
}

fn map_request_error(error: reqwest::Error) -> SkillMarketProviderError {
    if error.is_timeout() {
        SkillMarketProviderError::Timeout
    } else {
        SkillMarketProviderError::Upstream("SkillHub request could not be completed".to_string())
    }
}

fn ensure_success(status: StatusCode) -> Result<(), SkillMarketProviderError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(SkillMarketProviderError::Upstream(format!(
            "SkillHub returned HTTP status {}",
            status.as_u16()
        )))
    }
}

fn map_skill(raw: RawSkill) -> Result<SkillMarketSkillDto, SkillMarketProviderError> {
    let namespace = raw.namespace.handle().ok_or_else(|| {
        SkillMarketProviderError::InvalidResponse(
            "SkillHub skill is missing its namespace".to_string(),
        )
    })?;
    let slug = required(raw.slug, "slug")?;
    let version = required(raw.version, "version")?;
    let author = raw.owner_name.unwrap_or_else(|| namespace.clone());
    let tags = tags(raw.tags, raw.sub_categories);

    Ok(SkillMarketSkillDto {
        coordinate: SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto { source: SOURCE.to_string(), namespace, slug },
            version,
        },
        name: raw.name.unwrap_or_default(),
        description: raw.description.or(raw.description_zh).unwrap_or_default(),
        author,
        category: raw.category.unwrap_or_default(),
        tags,
        icon: raw.icon_url,
        downloads: raw.downloads.unwrap_or(0),
        installs: raw.installs.unwrap_or(0),
        rating: raw.score.unwrap_or_else(|| raw.stars.unwrap_or(0) as f64),
        updated_at: scalar_string(raw.updated_at),
    })
}

fn map_skill_detail(
    raw: RawSkillDetail,
) -> Result<SkillMarketSkillDetailDto, SkillMarketProviderError> {
    let namespace = raw.namespace.handle().ok_or_else(|| {
        SkillMarketProviderError::InvalidResponse(
            "SkillHub skill detail is missing its namespace".to_string(),
        )
    })?;
    let slug = required(raw.skill.slug, "slug")?;
    let version = required(raw.latest_version.version, "version")?;
    let requires_api_key = label_is_true(raw.skill.labels.as_ref(), "requires_api_key");
    let security = raw.security_reports.into_values().next().map(|report| SkillMarketSecurityDto {
        status: report.status.unwrap_or_else(|| "unknown".to_string()),
        summary: report.status_text,
    });
    let stats = raw.skill.stats.unwrap_or_default();

    Ok(SkillMarketSkillDetailDto {
        skill: SkillMarketSkillDto {
            coordinate: SkillMarketCoordinateDto {
                locator: SkillMarketSkillLocatorDto { source: SOURCE.to_string(), namespace, slug },
                version,
            },
            name: raw.skill.display_name.unwrap_or_default(),
            description: raw.skill.summary.or(raw.skill.summary_zh).unwrap_or_default(),
            author: raw.owner.display_name.or(raw.owner.handle).unwrap_or_default(),
            category: raw.skill.category.unwrap_or_default(),
            tags: tags(None, raw.skill.sub_categories),
            icon: raw.skill.icon_url,
            downloads: stats.downloads.unwrap_or(0),
            installs: stats.installs.unwrap_or(0),
            rating: stats.stars.unwrap_or(0) as f64,
            updated_at: scalar_string(raw.skill.updated_at),
        },
        license: raw.skill.license,
        homepage: raw.skill.homepage,
        repository: raw.skill.source_url,
        requires_api_key,
        security,
    })
}

fn map_skill_set(raw: RawSkillSet) -> SkillMarketSkillSetDto {
    SkillMarketSkillSetDto {
        source: SOURCE.to_string(),
        slug: raw.slug.unwrap_or_default(),
        name: raw.display_name.unwrap_or_default(),
        description: raw.summary.unwrap_or_default(),
        author: raw.author.unwrap_or_else(|| DISPLAY_NAME.to_string()),
        icon: raw.icon_url,
        member_count: raw
            .skill_count
            .unwrap_or_else(|| u64::try_from(raw.skills.len()).unwrap_or(u64::MAX)),
        updated_at: scalar_string(raw.updated_at),
    }
}

fn required(value: Option<String>, field: &str) -> Result<String, SkillMarketProviderError> {
    value.filter(|value| !value.is_empty()).ok_or_else(|| {
        SkillMarketProviderError::InvalidResponse(format!("SkillHub response is missing {field}"))
    })
}

fn required_value<'a>(value: &'a str, field: &str) -> Result<&'a str, SkillMarketProviderError> {
    if value.trim().is_empty() {
        Err(SkillMarketProviderError::InvalidResponse(format!(
            "skill coordinate is missing {field}"
        )))
    } else {
        Ok(value)
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

fn sort_value(sort: SkillMarketSort) -> &'static str {
    match sort {
        SkillMarketSort::Score => "score",
        SkillMarketSort::Downloads => "downloads",
        SkillMarketSort::UpdatedAt => "updated_at",
        SkillMarketSort::Installs => "installs",
    }
}

fn scalar_string(value: Option<Value>) -> String {
    match value {
        Some(Value::String(value)) => value,
        Some(Value::Number(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn tags(raw_tags: Option<Value>, sub_categories: Vec<RawCategoryRef>) -> Vec<String> {
    let tags = raw_tags
        .and_then(|value| value.as_array().cloned())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    if tags.is_empty() {
        sub_categories.into_iter().map(|category| category.key).collect()
    } else {
        tags
    }
}

fn label_is_true(labels: Option<&BTreeMap<String, Value>>, name: &str) -> bool {
    labels.and_then(|labels| labels.get(name)).is_some_and(|value| {
        value.as_bool().unwrap_or_else(|| {
            value.as_str().is_some_and(|value| value.eq_ignore_ascii_case("true"))
        })
    })
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize)]
struct RawEnvelope<T> {
    code: i32,
    data: Option<T>,
}

impl<T> RawEnvelope<T> {
    fn into_data(self) -> Result<T, SkillMarketProviderError> {
        if self.code != 0 {
            return Err(SkillMarketProviderError::Upstream(format!(
                "SkillHub returned business error code {}",
                self.code
            )));
        }
        self.data.ok_or_else(|| {
            SkillMarketProviderError::InvalidResponse(
                "SkillHub response is missing data".to_string(),
            )
        })
    }
}

#[derive(Deserialize)]
struct RawRecommended {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    skills: Vec<RawSkill>,
}

#[derive(Deserialize)]
struct RawSkillPage {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    skills: Vec<RawSkill>,
    #[serde(default)]
    total: u64,
}

#[derive(Deserialize)]
struct RawSkill {
    category: Option<String>,
    description: Option<String>,
    description_zh: Option<String>,
    downloads: Option<u64>,
    #[serde(rename = "iconUrl", alias = "icon_url")]
    icon_url: Option<String>,
    installs: Option<u64>,
    name: Option<String>,
    namespace: RawNamespace,
    #[serde(rename = "ownerName", alias = "owner_name")]
    owner_name: Option<String>,
    score: Option<f64>,
    slug: Option<String>,
    stars: Option<u64>,
    #[serde(
        rename = "subCategories",
        alias = "sub_categories",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    sub_categories: Vec<RawCategoryRef>,
    tags: Option<Value>,
    #[serde(rename = "updated_at", alias = "updatedAt")]
    updated_at: Option<Value>,
    version: Option<String>,
}

#[derive(Deserialize)]
struct RawNamespace {
    handle: Option<String>,
    #[serde(rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
    #[serde(rename = "canonicalName", alias = "canonical_name")]
    canonical_name: Option<String>,
}

impl RawNamespace {
    fn handle(&self) -> Option<String> {
        self.handle
            .clone()
            .or_else(|| self.display_name.clone())
            .or_else(|| {
                self.canonical_name
                    .as_deref()
                    .and_then(|value| value.strip_prefix('@'))
                    .and_then(|value| value.split('/').next())
                    .map(str::to_string)
            })
            .filter(|value| !value.is_empty())
    }
}

#[derive(Deserialize)]
struct RawCategoryRef {
    key: String,
}

#[derive(Deserialize)]
struct RawCategories {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    items: Vec<RawCategory>,
}

#[derive(Deserialize)]
struct RawCategory {
    key: String,
    name: String,
    #[serde(rename = "skillCount", alias = "skill_count")]
    skill_count: Option<u64>,
}

#[derive(Deserialize)]
struct RawSkillDetail {
    #[serde(rename = "latestVersion", alias = "latest_version")]
    latest_version: RawLatestVersion,
    namespace: RawNamespace,
    owner: RawOwner,
    #[serde(
        rename = "securityReports",
        alias = "security_reports",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    security_reports: BTreeMap<String, RawSecurityReport>,
    skill: RawDetailSkill,
}

#[derive(Deserialize)]
struct RawLatestVersion {
    version: Option<String>,
}

#[derive(Deserialize)]
struct RawOwner {
    #[serde(rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
    handle: Option<String>,
}

#[derive(Deserialize)]
struct RawSecurityReport {
    status: Option<String>,
    #[serde(rename = "statusText", alias = "status_text")]
    status_text: Option<String>,
}

#[derive(Deserialize)]
struct RawDetailSkill {
    category: Option<String>,
    #[serde(rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
    homepage: Option<String>,
    #[serde(rename = "iconUrl", alias = "icon_url")]
    icon_url: Option<String>,
    labels: Option<BTreeMap<String, Value>>,
    license: Option<String>,
    slug: Option<String>,
    #[serde(rename = "sourceUrl", alias = "source_url")]
    source_url: Option<String>,
    stats: Option<RawStats>,
    #[serde(
        rename = "subCategories",
        alias = "sub_categories",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    sub_categories: Vec<RawCategoryRef>,
    summary: Option<String>,
    summary_zh: Option<String>,
    #[serde(rename = "updatedAt", alias = "updated_at")]
    updated_at: Option<Value>,
}

#[derive(Default, Deserialize)]
struct RawStats {
    downloads: Option<u64>,
    installs: Option<u64>,
    stars: Option<u64>,
}

#[derive(Deserialize)]
struct RawSkillSetPage {
    #[serde(
        rename = "skillSets",
        alias = "skill_sets",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    skill_sets: Vec<RawSkillSet>,
    #[serde(default)]
    total: u64,
}

#[derive(Deserialize)]
struct RawSkillSet {
    slug: Option<String>,
    #[serde(rename = "displayName", alias = "display_name")]
    display_name: Option<String>,
    summary: Option<String>,
    author: Option<String>,
    #[serde(rename = "iconUrl", alias = "icon_url")]
    icon_url: Option<String>,
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    skills: Vec<RawSkillSetMember>,
    #[serde(rename = "skillCount", alias = "skill_count")]
    skill_count: Option<u64>,
    #[serde(rename = "updatedAt", alias = "updated_at")]
    updated_at: Option<Value>,
}

#[derive(Deserialize)]
struct RawSkillSetMember {
    slug: String,
    // Unresolved catalog members have no namespace; installation validates it separately.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    namespace: String,
    version: Option<String>,
}

#[derive(Deserialize)]
struct RawArtifactSignature {
    content_hash: String,
    key_id: Option<String>,
    payload: Option<String>,
    signature: Option<String>,
    signed: bool,
}

#[derive(Deserialize)]
struct RawPlatformKeys {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    keys: Vec<RawPlatformKey>,
}

#[derive(Deserialize)]
struct RawPlatformKey {
    key_id: String,
    algorithm: String,
    public_key_raw_b64: String,
}
