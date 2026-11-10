//! 读取员工登录 Key 对应的月度模型额度。

use std::time::Duration;

use serde::Deserialize;
use vw_gateway_client::ShijuMonthlyQuotaDto;

use super::{ShijuAccountClient, ShijuAccountError, endpoint};

const REQUEST_FAILED: &str = "识矩月度额度读取失败";
const INVALID_RESPONSE: ShijuAccountError = ShijuAccountError::Message("识矩月度额度响应无效");

#[derive(Deserialize)]
struct UsageEnvelope {
    #[serde(default = "success_code")]
    code: i64,
    data: MonthlyQuota,
}

fn success_code() -> i64 {
    200
}

#[derive(Deserialize)]
struct MonthlyQuota {
    monthly_quota: u64,
    monthly_used_quota: u64,
    monthly_reset_at: i64,
    quota_per_unit: f64,
}

impl MonthlyQuota {
    fn into_dto(self) -> Result<ShijuMonthlyQuotaDto, ShijuAccountError> {
        let unit = self.quota_per_unit;
        let limit = self.monthly_quota as f64 / unit;
        let used = self.monthly_used_quota as f64 / unit;
        if !unit.is_finite()
            || unit <= 0.0
            || !limit.is_finite()
            || !used.is_finite()
            || self.monthly_reset_at <= 0
            || chrono::DateTime::from_timestamp(self.monthly_reset_at, 0).is_none()
        {
            return Err(INVALID_RESPONSE);
        }
        let remaining = (self.monthly_quota != 0)
            .then(|| self.monthly_quota.saturating_sub(self.monthly_used_quota) as f64 / unit);
        Ok(ShijuMonthlyQuotaDto { limit, used, remaining, reset_at: self.monthly_reset_at })
    }
}

impl ShijuAccountClient {
    pub(crate) async fn monthly_quota(
        &self,
        credential: &str,
    ) -> Result<ShijuMonthlyQuotaDto, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        let failure = || ShijuAccountError::Message(REQUEST_FAILED);
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| failure())?;
        let response = http
            .get(endpoint(&self.gateway_base, "/model/api/runtime/model-usage")?)
            .query(&[("include_monthly_quota", true)])
            .header("X-Api-Key", credential)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| failure())?;
        if response.status().is_redirection() {
            return Err(failure());
        }
        if !response.status().is_success() {
            return Err(ShijuAccountError::Upstream {
                status: response.status(),
                message: REQUEST_FAILED.to_string(),
            });
        }
        let envelope = response.json::<UsageEnvelope>().await.map_err(|_| INVALID_RESPONSE)?;
        if !matches!(envelope.code, 0 | 200) {
            return Err(INVALID_RESPONSE);
        }
        envelope.data.into_dto()
    }
}
