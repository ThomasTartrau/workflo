use tracing::info;

use workflo::Operation;

use crate::context::OrderContext;
use crate::error::OrderOperationError;

#[derive(Debug, Clone)]
pub struct ChargePayment {
    pub order_id: String,
    pub amount_cents: i64,
    pub currency: String,
}

impl Operation<OrderContext> for ChargePayment {
    type Error = OrderOperationError;

    fn name(&self) -> &str {
        "charge_payment"
    }

    async fn execute(self, ctx: &OrderContext) -> Result<Self, Self::Error> {
        info!(
            api = %ctx.payment_api,
            order_id = %self.order_id,
            amount_cents = self.amount_cents,
            currency = %self.currency,
            "charging payment"
        );
        Ok(self)
    }

    async fn rollback(self, _ctx: &OrderContext) -> Result<(), Self::Error> {
        info!(
            order_id = %self.order_id,
            amount_cents = self.amount_cents,
            "refunding payment"
        );
        Ok(())
    }

    async fn commit(self, _ctx: &OrderContext) -> Result<(), Self::Error> {
        info!(order_id = %self.order_id, "payment capture confirmed");
        Ok(())
    }
}
