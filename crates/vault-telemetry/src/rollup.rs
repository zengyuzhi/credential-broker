#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub estimated_cost_usd: f64,
}
