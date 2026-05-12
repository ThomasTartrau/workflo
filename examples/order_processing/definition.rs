use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDefinition {
    pub order_id: String,
    pub item_id: String,
    pub quantity: u32,
    pub amount_cents: i64,
    pub currency: String,
    pub shipping_address: String,
    pub current_step: OrderStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStep {
    ReserveStock,
    ChargePayment,
    Ship,
    Done,
}
