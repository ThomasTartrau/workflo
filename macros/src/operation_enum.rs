use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

struct OperationEnumArgs {
    context: Type,
    error: Type,
}

impl syn::parse::Parse for OperationEnumArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut context = None;
        let mut error = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let ty: Type = input.parse()?;

            match ident.to_string().as_str() {
                "context" => context = Some(ty),
                "error" => error = Some(ty),
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown parameter `{other}`, expected `context` or `error`"),
                    ))
                }
            }

            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Self {
            context: context.ok_or_else(|| input.error("missing `context` parameter"))?,
            error: error.ok_or_else(|| input.error("missing `error` parameter"))?,
        })
    }
}

/// Derive `Operation` dispatch for an enum wrapping individual operation types.
///
/// Each variant must be a newtype: `Variant(InnerType)` where `InnerType`
/// implements `Operation<Ctx>` with the same error type.
///
/// Requires `#[operation(context = Type, error = Type)]` on the enum.
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input, "OperationEnum can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("operation")) else {
        return syn::Error::new_spanned(
            &input,
            "OperationEnum requires #[operation(context = Type, error = Type)]",
        )
        .to_compile_error()
        .into();
    };

    let args: OperationEnumArgs = match attr.parse_args() {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut variant_idents = Vec::new();

    for variant in &data_enum.variants {
        let Fields::Unnamed(fields) = &variant.fields else {
            return syn::Error::new_spanned(
                variant,
                "OperationEnum variants must be newtype variants: Variant(InnerType)",
            )
            .to_compile_error()
            .into();
        };
        if fields.unnamed.len() != 1 {
            return syn::Error::new_spanned(
                variant,
                "OperationEnum variants must have exactly one field",
            )
            .to_compile_error()
            .into();
        }
        variant_idents.push(&variant.ident);
    }

    let ctx_type = &args.context;
    let err_type = &args.error;

    let name_arms = variant_idents.iter().map(|v| {
        quote! { Self::#v(inner) => inner.name() }
    });

    let execute_arms = variant_idents.iter().map(|v| {
        quote! { Self::#v(inner) => inner.execute(ctx).await.map(Self::#v) }
    });

    let rollback_arms = variant_idents.iter().map(|v| {
        quote! { Self::#v(inner) => inner.rollback(ctx).await }
    });

    let commit_arms = variant_idents.iter().map(|v| {
        quote! { Self::#v(inner) => inner.commit(ctx).await }
    });

    quote! {
        impl ::workflo::Operation<#ctx_type> for #name {
            type Error = #err_type;

            fn name(&self) -> &str {
                match self {
                    #(#name_arms,)*
                }
            }

            fn execute(
                self,
                ctx: &#ctx_type,
            ) -> impl ::std::future::Future<Output = ::std::result::Result<Self, Self::Error>> + Send
            {
                async move {
                    match self {
                        #(#execute_arms,)*
                    }
                }
            }

            fn rollback(
                self,
                ctx: &#ctx_type,
            ) -> impl ::std::future::Future<Output = ::std::result::Result<(), Self::Error>> + Send
            {
                async move {
                    match self {
                        #(#rollback_arms,)*
                    }
                }
            }

            fn commit(
                self,
                ctx: &#ctx_type,
            ) -> impl ::std::future::Future<Output = ::std::result::Result<(), Self::Error>> + Send
            {
                async move {
                    match self {
                        #(#commit_arms,)*
                    }
                }
            }
        }
    }
    .into()
}
