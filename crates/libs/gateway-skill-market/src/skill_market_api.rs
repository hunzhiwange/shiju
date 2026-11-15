use serde::de::DeserializeOwned;
use serde::Deserialize;
use super::{GatewaySkillMarketClient, types::*};

#[derive(Deserialize)]
struct SkillPath {
    path: String,
}

impl GatewaySkillMarketClient {
    /// Lists the skill-market sources registered by the gateway.
    pub async fn skill_market_sources(&self) -> Result<Vec<SkillMarketSourceDto>, String> {
        self.skill_market_get("/v1/skill-market/sources", &[]).await
    }

    /// Lists the recommended skills from one registered source.
    pub async fn skill_market_recommended(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketSkillDto>, String> {
        self.skill_market_get(
            "/v1/skill-market/showcase/recommended",
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Queries the unified skill catalog for one registered source.
    pub async fn skill_market_skills(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillPageDto, String> {
        self.skill_market_get("/v1/skill-market/skills", &skill_market_catalog_query(source, query))
            .await
    }

    /// Lists the unified categories exposed by one registered source.
    pub async fn skill_market_categories(
        &self,
        source: &str,
    ) -> Result<Vec<SkillMarketCategoryDto>, String> {
        self.skill_market_get(
            "/v1/skill-market/categories",
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Reads one source-aware skill detail.
    pub async fn skill_market_skill_detail(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillDetailDto, String> {
        self.skill_market_get(
            &format!("/v1/skill-market/skills/{}", encode_skill_market_path_segment(&locator.slug)),
            &skill_market_locator_query(locator),
        )
        .await
    }

    /// Reads the canonical Markdown file for one source-aware skill.
    pub async fn skill_market_skill_file(
        &self,
        locator: &SkillMarketSkillLocatorDto,
    ) -> Result<SkillMarketSkillFileDto, String> {
        self.skill_market_get(
            &format!(
                "/v1/skill-market/skills/{}/file",
                encode_skill_market_path_segment(&locator.slug)
            ),
            &skill_market_locator_query(locator),
        )
        .await
    }

    /// Installs one canonical source coordinate into the current project.
    pub async fn skill_market_install(
        &self,
        project_path: &str,
        coordinate: &SkillMarketCoordinateDto,
    ) -> Result<String, String> {
        let request = SkillMarketInstallRequest {
            project_path: project_path.to_string(),
            source: coordinate.locator.source.clone(),
            namespace: coordinate.locator.namespace.clone(),
            slug: coordinate.locator.slug.clone(),
            version: coordinate.version.clone(),
        };
        let envelope: SkillMarketEnvelope<SkillPath> =
            self.transport.post_json("/v1/skill-market/skills/install", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.path)
    }

    pub async fn skill_market_install_to_targets(
        &self,
        project_path: Option<&str>,
        coordinate: &SkillMarketCoordinateDto,
        installations: &[SkillMarketInstallSelectionDto],
    ) -> Result<Vec<String>, String> {
        let request = SkillMarketMultiInstallRequest {
            project_path: project_path.unwrap_or_default().to_string(),
            source: coordinate.locator.source.clone(),
            namespace: coordinate.locator.namespace.clone(),
            slug: coordinate.locator.slug.clone(),
            version: coordinate.version.clone(),
            installations: installations.to_vec(),
        };
        let envelope: SkillMarketEnvelope<Vec<SkillPath>> =
            self.transport.post_json("/v1/skill-market/skills/install-targets", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.into_iter().map(|item| item.path).collect())
    }

    /// Queries skill sets from one registered source.
    pub async fn skill_market_skill_sets(
        &self,
        source: &str,
        query: &SkillMarketSkillQuery,
    ) -> Result<SkillMarketSkillSetPageDto, String> {
        self.skill_market_get(
            "/v1/skill-market/skillsets",
            &skill_market_catalog_query(source, query),
        )
        .await
    }

    /// Reads one skill-set detail from a registered source.
    pub async fn skill_market_skill_set_detail(
        &self,
        source: &str,
        slug: &str,
    ) -> Result<SkillMarketSkillSetDetailDto, String> {
        self.skill_market_get(
            &format!("/v1/skill-market/skillsets/{}", encode_skill_market_path_segment(slug)),
            &[("source".to_string(), source.to_string())],
        )
        .await
    }

    /// Installs one source-aware skill set as a single atomic transaction.
    pub async fn skill_market_skill_set_install(
        &self,
        project_path: &str,
        source: &str,
        slug: &str,
    ) -> Result<Vec<String>, String> {
        let request = SkillMarketSkillSetInstallRequest {
            project_path: project_path.to_string(),
            source: source.to_string(),
            slug: slug.to_string(),
        };
        let envelope: SkillMarketEnvelope<Vec<SkillPath>> =
            self.transport.post_json("/v1/skill-market/skillsets/install", &[], &request).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data.into_iter().map(|item| item.path).collect())
    }

    async fn skill_market_get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T, String> {
        let envelope: SkillMarketEnvelope<T> = self.transport.get_json(path, query).await?;
        if envelope.code != 0 {
            return Err(format!(
                "skill market request failed (code {}): {}",
                envelope.code, envelope.message
            ));
        }
        Ok(envelope.data)
    }
}

fn skill_market_catalog_query(
    source: &str,
    query: &SkillMarketSkillQuery,
) -> Vec<(String, String)> {
    let mut pairs = vec![("source".to_string(), source.to_string())];
    if let Some(keyword) = &query.keyword {
        pairs.push(("keyword".to_string(), keyword.clone()));
    }
    if let Some(category) = &query.category {
        pairs.push(("category".to_string(), category.clone()));
    }
    if let Some(namespace) = &query.namespace {
        pairs.push(("namespace".to_string(), namespace.clone()));
    }
    pairs.extend([
        ("sort".to_string(), skill_market_sort_value(query.sort).to_string()),
        ("page".to_string(), query.page.to_string()),
        ("page_size".to_string(), query.page_size.to_string()),
    ]);
    pairs
}

fn skill_market_locator_query(locator: &SkillMarketSkillLocatorDto) -> Vec<(String, String)> {
    vec![
        ("source".to_string(), locator.source.clone()),
        ("namespace".to_string(), locator.namespace.clone()),
    ]
}

const fn skill_market_sort_value(sort: SkillMarketSort) -> &'static str {
    match sort {
        SkillMarketSort::Score => "score",
        SkillMarketSort::Downloads => "downloads",
        SkillMarketSort::UpdatedAt => "updated_at",
        SkillMarketSort::Installs => "installs",
    }
}

fn encode_skill_market_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
