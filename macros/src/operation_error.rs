use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Derive `OperationError` for error enums.
///
/// Variant attributes:
/// - `#[soft]`: soft failure, retries without counting (`consume_retry = false`)
/// - `#[invariant]`: unrecoverable, no retry (`is_violated_invariant = true`)
/// - No attribute: standard retriable error (`consume_retry = true`, `is_violated_invariant = false`)
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input, "OperationError can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut consume_retry_arms = Vec::new();
    let mut invariant_arms = Vec::new();

    for variant in &data_enum.variants {
        let ident = &variant.ident;
        let has_soft = variant.attrs.iter().any(|a| a.path().is_ident("soft"));
        let has_invariant = variant.attrs.iter().any(|a| a.path().is_ident("invariant"));

        if has_soft && has_invariant {
            return syn::Error::new_spanned(
                variant,
                "variant cannot be both #[soft] and #[invariant]",
            )
            .to_compile_error()
            .into();
        }

        let pattern = match &variant.fields {
            Fields::Named(_) => quote! { Self::#ident { .. } },
            Fields::Unnamed(_) => quote! { Self::#ident(..) },
            Fields::Unit => quote! { Self::#ident },
        };

        let consume = !has_soft;
        consume_retry_arms.push(quote! { #pattern => #consume });
        invariant_arms.push(quote! { #pattern => #has_invariant });
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics ::workflo::OperationError for #name #ty_generics #where_clause {
            fn consume_retry(&self) -> bool {
                match self {
                    #(#consume_retry_arms,)*
                }
            }

            fn is_violated_invariant(&self) -> bool {
                match self {
                    #(#invariant_arms,)*
                }
            }
        }
    }
    .into()
}
