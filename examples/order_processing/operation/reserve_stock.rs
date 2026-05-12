use tracing::info;

use workflo::Operation;

use crate::context::OrderContext;
use crate::error::OrderOperationError;

#[derive(Debug, Clone)]
pub struct ReserveStock {
    pub order_id: String,
    pub item_id: String,
    pub quantity: u32,
}

impl Operation<OrderContext> for ReserveStock {
    type Error = OrderOperationError;

    fn name(&self) -> &str {
        "reserve_stock"
    }

    async fn execute(self, ctx: &OrderContext) -> Result<Self, Self::Error> {
        info!(
            api = %ctx.inventory_api,
            order_id = %self.order_id,
            item_id = %self.item_id,
            quantity = self.quantity,
            "reserving stock"
        );
        Ok(self)
    }

    async fn rollback(self, _ctx: &OrderContext) -> Result<(), Self::Error> {
        info!(
            order_id = %self.order_id,
            item_id = %self.item_id,
            quantity = self.quantity,
            "releasing reserved stock"
        );
        Ok(())
    }

    async fn commit(self, _ctx: &OrderContext) -> Result<(), Self::Error> {
        info!(order_id = %self.order_id, "stock reservation confirmed");
        Ok(())
    }
}
