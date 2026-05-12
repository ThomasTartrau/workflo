use tracing::info;

use workflo::Operation;

use crate::context::OrderContext;
use crate::error::OrderOperationError;

#[derive(Debug, Clone)]
pub struct ShipOrder {
    pub order_id: String,
    pub address: String,
}

impl Operation<OrderContext> for ShipOrder {
    type Error = OrderOperationError;

    fn name(&self) -> &str {
        "ship_order"
    }

    async fn execute(self, ctx: &OrderContext) -> Result<Self, Self::Error> {
        info!(
            api = %ctx.shipping_api,
            order_id = %self.order_id,
            address = %self.address,
            "creating shipment"
        );
        Ok(self)
    }
}
