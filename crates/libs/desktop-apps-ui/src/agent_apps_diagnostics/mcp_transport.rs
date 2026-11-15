use vw_gateway_client::GatewayClient;
#[derive(Clone)]
pub struct ProbeTransport {
    pub client: GatewayClient,
    pub token: String,
}
impl std::fmt::Debug for ProbeTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProbeTransport").finish_non_exhaustive()
    }
}
