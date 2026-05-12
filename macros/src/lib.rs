mod operation_enum;
mod operation_error;

use proc_macro::TokenStream;

#[proc_macro_derive(OperationError, attributes(soft, invariant))]
pub fn derive_operation_error(input: TokenStream) -> TokenStream {
    operation_error::derive(input)
}

#[proc_macro_derive(OperationEnum, attributes(operation))]
pub fn derive_operation_enum(input: TokenStream) -> TokenStream {
    operation_enum::derive(input)
}
