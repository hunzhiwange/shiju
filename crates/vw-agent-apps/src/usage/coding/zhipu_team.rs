//! Zhipu Team Coding Plan quota queries against the official domestic endpoint.

use reqwest::{
    ClientBuilder,
    header::{ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_TYPE, HeaderValue},
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{UsageQueryErrorCode, UsageQueryResult},
};

use super::zhipu::{
    CN_ORIGIN, PLAN_PATH, PlanResponse, authorization, client_builder, parse_windows, plan_result,
    read_json, transport_error,
};
use crate::{providers::validation::header_value, usage::detect::CodingPlanProvider};

pub struct ZhipuTeamCredentials<'a> {
    pub api_key: &'a str,
    pub organization_id: Option<&'a str>,
    pub project_id: Option<&'a str>,
}

/// Queries only a provider resolved through explicit team selection. The caller must authorize
/// the query and resolve the credentials on the execution host.
pub async fn query_zhipu_team(
    provider: CodingPlanProvider,
    credentials: ZhipuTeamCredentials<'_>,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    query_at(client_builder(), CN_ORIGIN, provider, credentials, queried_at, correlation_id).await
}

async fn query_at(
    builder: ClientBuilder,
    origin: &str,
    provider: CodingPlanProvider,
    credentials: ZhipuTeamCredentials<'_>,
    queried_at: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    let request = (|| {
        if provider != CodingPlanProvider::ZhipuTeam {
            return Err(UsageQueryErrorCode::Unsupported);
        }
        let authorization = authorization(credentials.api_key)?;
        let organization = team_header(credentials.organization_id)?;
        let project = team_header(credentials.project_id)?;
        let client = builder.build().map_err(transport_error)?;
        Ok(client
            .get(format!("{origin}{PLAN_PATH}?type=2"))
            .header(AUTHORIZATION, authorization)
            .header("bigmodel-organization", organization)
            .header("bigmodel-project", project)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT_LANGUAGE, "en-US,en"))
    })();

    let (queried_at, result) = match request {
        Ok(request) => {
            // An upstream rejection remains a team failure even when it reports no plan.
            let result = read_json::<PlanResponse>(request)
                .await
                .and_then(PlanResponse::into_data)
                .and_then(parse_windows);
            (Some(queried_at), result)
        }
        Err(code) => (None, Err(code)),
    };
    plan_result("zhipu_team", queried_at, correlation_id, result)
}

fn team_header(value: Option<&str>) -> Result<HeaderValue, UsageQueryErrorCode> {
    use UsageQueryErrorCode::MissingConfiguration;

    let value = value.ok_or(MissingConfiguration)?;
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(MissingConfiguration);
    }
    let mut header = header_value(value.trim()).map_err(|_| MissingConfiguration)?;
    header.set_sensitive(true);
    Ok(header)
}

#[cfg(test)]
#[path = "zhipu_team_tests.rs"]
mod tests;
