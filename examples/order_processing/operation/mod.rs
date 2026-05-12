pub mod charge_payment;
pub mod reserve_stock;
pub mod ship_order;

use workflo::OperationEnum;

use crate::context::OrderContext;
use crate::error::OrderOperationError;
use charge_payment::ChargePayment;
use reserve_stock::ReserveStock;
use ship_order::ShipOrder;

#[derive(Debug, Clone, OperationEnum)]
#[operation(context = OrderContext, error = OrderOperationError)]
pub enum OrderOp {
    ReserveStock(ReserveStock),
    ChargePayment(ChargePayment),
    ShipOrder(ShipOrder),
}
