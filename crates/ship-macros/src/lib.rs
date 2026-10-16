//! Ship's procedural macros. Generated code names dependencies by absolute
//! path, so the using crate must depend on each crate a macro uses.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, GenericArgument, ImplItem, ItemFn, ItemImpl, LitStr, Meta, Path, PathArguments,
    ReturnType, Token, Type, parse_macro_input, punctuated::Punctuated,
};

/// Instruments an operation at debug with arguments skipped. Functions returning
/// `Result` log errors at warn using Debug. `no_err` disables error logging;
/// other options pass through to `tracing::instrument`, which validates them.
/// The emitting crate must depend on `tracing`.
#[proc_macro_attribute]
pub fn traced(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let function = parse_macro_input!(item as ItemFn);
    traced_function(args, function).into()
}

fn traced_function(
    args: Punctuated<Meta, Token![,]>,
    function: ItemFn,
) -> proc_macro2::TokenStream {
    let mut options = Vec::new();
    let mut level = false;
    let mut no_err = false;
    for arg in args {
        if arg.path().is_ident("no_err") && matches!(arg, Meta::Path(_)) {
            no_err = true;
        } else {
            level |= arg.path().is_ident("level");
            options.push(arg);
        }
    }
    let default_level = (!level).then(|| quote!(level = "debug",));
    let result = matches!(&function.sig.output,
        ReturnType::Type(_, ty) if matches!(ty.as_ref(), Type::Path(path)
            if path.path.segments.last().is_some_and(|segment| segment.ident == "Result")));
    let error = (result && !no_err).then(|| quote!(err(level = "warn", Debug),));
    quote! {
        #[::tracing::instrument(#default_level skip_all, #error #(#options),*)]
        #function
    }
}

/// Gives `impl Message<X>` the short message name `X`, including its type
/// arguments, instead of Kameo's fully qualified default name.
#[proc_macro_attribute]
pub fn named(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "`named` takes no arguments")
            .into_compile_error()
            .into();
    }
    let implementation = parse_macro_input!(item as ItemImpl);
    named_message(implementation)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn named_message(mut implementation: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    let message = implementation
        .trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last())
        .filter(|segment| segment.ident == "Message")
        .and_then(|segment| match &segment.arguments {
            PathArguments::AngleBracketed(args) => args.args.first(),
            _ => None,
        })
        .and_then(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .ok_or_else(|| syn::Error::new_spanned(&implementation, "expected `impl Message<X>`"))?;
    let name = quote!(#message).to_string().replace(' ', "");
    let method: ImplItem = syn::parse_quote! {
        fn name() -> &'static str { #name }
    };
    implementation.items.push(method);
    Ok(quote!(#implementation))
}

/// Kameo's `#[derive(Actor)]` with Ship's defaults: `Args = Self`,
/// `Error = Infallible` and an `on_start` that returns the state, plus an
/// `on_panic` that keeps the actor running when a handler returns `Err`.
///
/// Kameo reports an `Err` from a `tell`'d handler as a panic with reason
/// `OnMessage`, and its default `on_panic` stops the actor. There is no caller
/// to return that error to, so this logs it and continues. A real panic still
/// stops the actor. `#[actor(name = "...")]` overrides the actor name, as in
/// Kameo. `#[actor(on_stop = path)]` forwards Kameo's `on_stop` to the
/// function at `path`, which takes the same arguments and returns the same
/// result. The deriving crate must depend on `kameo` and `tracing`.
#[proc_macro_derive(Actor, attributes(actor))]
pub fn derive_actor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    actor(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn actor(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut name = None;
    let mut on_stop = None;
    for attr in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("actor"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if name.is_some() {
                    return Err(meta.error("name already set"));
                }
                name = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("on_stop") {
                if on_stop.is_some() {
                    return Err(meta.error("on_stop already set"));
                }
                on_stop = Some(meta.value()?.parse::<Path>()?);
            } else {
                return Err(meta.error("expected `name` or `on_stop`"));
            }
            Ok(())
        })?;
    }
    let ident = &input.ident;
    let name = name.unwrap_or_else(|| ident.to_string());
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let on_stop = on_stop.map(|path| {
        quote! {
            async fn on_stop(
                &mut self,
                actor_ref: ::kameo::actor::WeakActorRef<Self>,
                reason: ::kameo::error::ActorStopReason,
            ) -> ::std::result::Result<(), Self::Error> {
                #path(self, actor_ref, reason).await
            }
        }
    });
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

            #on_stop
        }
    })
}
