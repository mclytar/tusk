use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal};
use quote::quote;
use syn::{ImplItem, ItemImpl, parse_macro_input};

#[proc_macro_attribute]
pub fn rest_resource(args: TokenStream, input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as Literal);
    let body = parse_macro_input!(input as ItemImpl);

    let self_ty = body.self_ty.clone();
    let methods: Vec<Ident> = body.items.iter()
        .filter_map(|item| {
            match item {
                ImplItem::Fn(method) => match method.sig.ident.to_string().as_str() {
                    "get" | "post" | "put" | "head" | "delete" | "patch" | "options" | "connect" | "trace" => Some(method.sig.ident.clone()),
                    _ => None
                },
                _ => None
            }
        }).collect();

    quote! {
        #body

        impl actix_web::dev::HttpServiceFactory for #self_ty {
            fn register(self, config: &mut actix_web::dev::AppService) {
                actix_web::web::resource(#path)
                    #(.route(actix_web::web::#methods().to(Self::#methods)))*
                    .register(config)
            }
        }
    }.into()
}
