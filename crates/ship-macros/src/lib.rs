//! Ship's own derives. Generated code names its dependencies by absolute
//! path, so the deriving crate must depend on each crate a derive uses.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, parse_macro_input};

/// Kameo's `#[derive(Actor)]` with Ship's defaults: `Args = Self`,
/// `Error = Infallible` and an `on_start` that returns the state, plus an
/// `on_panic` that keeps the actor running when a handler returns `Err`.
///
/// Kameo reports an `Err` from a `tell`'d handler as a panic with reason
/// `OnMessage`, and its default `on_panic` stops the actor. There is no caller
/// to return that error to, so this logs it and continues. A real panic still
/// stops the actor. `#[actor(name = "...")]` overrides the actor name, as in
/// Kameo. The deriving crate must depend on `kameo` and `tracing`.
#[proc_macro_derive(Actor, attributes(actor))]
pub fn derive_actor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    actor(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn actor(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut name = None;
    for attr in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("actor"))
    {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("name") {
                return Err(meta.error("expected `name`"));
            }
            if name.is_some() {
                return Err(meta.error("name already set"));
            }
            name = Some(meta.value()?.parse::<LitStr>()?.value());
            Ok(())
        })?;
    }
    let ident = &input.ident;
    let name = name.unwrap_or_else(|| ident.to_string());
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics ::kameo::actor::Actor for #ident #ty_generics #where_clause {
            type Args = Self;
            type Error = ::kameo::error::Infallible;

            fn name() -> &'static str {
                #name
            }

            async fn on_start(
                state: Self::Args,
                _actor_ref: ::kameo::actor::ActorRef<Self>,
            ) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(state)
            }

            async fn on_panic(
                &mut self,
                _actor_ref: ::kameo::actor::WeakActorRef<Self>,
                err: ::kameo::error::PanicError,
            ) -> ::std::result::Result<
                ::std::ops::ControlFlow<::kameo::error::ActorStopReason>,
                Self::Error,
            > {
                if err.reason() == ::kameo::error::PanicReason::OnMessage {
                    // `Debug`, because `Display` omits a non-string inner error.
                    ::tracing::warn!(actor = #name, error = ?err, "message handler failed");
                    return ::std::result::Result::Ok(::std::ops::ControlFlow::Continue(()));
                }
                ::std::result::Result::Ok(::std::ops::ControlFlow::Break(
                    ::kameo::error::ActorStopReason::Panicked(err),
                ))
            }
        }
    })
}
