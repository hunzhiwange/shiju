//! Project-scoped Pi and OMP directories. Native paths and profile selection stay remote.
use super::{AggregationApiError as Error, GatewayClient};
use vw_api_app_aggregation::AggregationQuery;
use vw_api_app_aggregation::OmpDirectoryState;
use vw_api_app_aggregation::OmpDirectoryWrite;
use vw_api_app_aggregation::PiDirectoryState;
use vw_api_app_aggregation::PiDirectoryWrite;

impl GatewayClient {
    /// Read Pi directory metadata without changing native defaults or login.
    pub async fn pi_directory_state(
        &self,
        query: &AggregationQuery,
    ) -> Result<PiDirectoryState, Error> {
        gateway_aggregation_native::GatewayAggregationNativeClient::new(self.transport.clone())
            .pi_directory_state(query).await
    }
    /// Apply one typed Pi entry change using the revision returned by the gateway.
    pub async fn pi_directory_entry(
        &self,
        id: &str,
        body: &PiDirectoryWrite,
    ) -> Result<PiDirectoryState, Error> {
        gateway_aggregation_native::GatewayAggregationNativeClient::new(self.transport.clone())
            .pi_directory_entry(id, body)
            .await
    }
    /// Read OMP metadata including the profile selected by the execution host.
    pub async fn omp_directory_state(
        &self,
        query: &AggregationQuery,
    ) -> Result<OmpDirectoryState, Error> {
        gateway_aggregation_native::GatewayAggregationNativeClient::new(self.transport.clone())
            .omp_directory_state(query).await
    }
    /// Apply one typed OMP entry change; the supplied profile is a provider declaration.
    pub async fn omp_directory_entry(
        &self,
        id: &str,
        body: &OmpDirectoryWrite,
    ) -> Result<OmpDirectoryState, Error> {
        gateway_aggregation_native::GatewayAggregationNativeClient::new(self.transport.clone())
            .omp_directory_entry(id, body)
            .await
    }
}
