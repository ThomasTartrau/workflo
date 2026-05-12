use workflo::{EngineError, Execution, WorkflowResolver};

use crate::context::OrderContext;
use crate::definition::{OrderDefinition, OrderStep};
use crate::operation::charge_payment::ChargePayment;
use crate::operation::reserve_stock::ReserveStock;
use crate::operation::ship_order::ShipOrder;
use crate::operation::OrderOp;

pub struct OrderResolver;

impl WorkflowResolver<OrderContext> for OrderResolver {
    type Op = OrderOp;

    async fn resolve(
        &self,
        execution: &mut Execution,
        _ctx: &OrderContext,
    ) -> Result<Vec<OrderOp>, EngineError> {
        let mut def: OrderDefinition = serde_json::from_value(execution.definition.clone())
            .map_err(|e| EngineError::Resolution(e.to_string()))?;

        let ops = match def.current_step {
            OrderStep::ReserveStock => {
                def.current_step = OrderStep::ChargePayment;
                vec![OrderOp::ReserveStock(ReserveStock {
                    order_id: def.order_id.clone(),
                    item_id: def.item_id.clone(),
                    quantity: def.quantity,
                })]
            }
            OrderStep::ChargePayment => {
                def.current_step = OrderStep::Ship;
                vec![OrderOp::ChargePayment(ChargePayment {
                    order_id: def.order_id.clone(),
                    amount_cents: def.amount_cents,
                    currency: def.currency.clone(),
                })]
            }
            OrderStep::Ship => {
                def.current_step = OrderStep::Done;
                vec![OrderOp::ShipOrder(ShipOrder {
                    order_id: def.order_id.clone(),
                    address: def.shipping_address.clone(),
                })]
            }
            OrderStep::Done => vec![],
        };

        execution.definition =
            serde_json::to_value(&def).map_err(|e| EngineError::Resolution(e.to_string()))?;

        Ok(ops)
    }
}
