use vst3_com_macros_support::aggr_co_class::expand_aggr_co_class;
use vst3_com_macros_support::co_class::expand_co_class;
use vst3_com_macros_support::com_interface::{expand_com_interface, expand_derive};

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{punctuated::Punctuated, Ident, ItemStruct, Meta, Token};

// All the Macro exports declared here. Delegates to respective crate for expansion.
#[proc_macro_attribute]
pub fn com_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_com_interface(attr, item)
}

#[proc_macro_derive(VTable)]
pub fn derive_vtable(input: TokenStream) -> TokenStream {
    expand_derive(input)
}

// Macro entry points.
#[proc_macro_attribute]
pub fn co_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemStruct);
    let attr_args =
        syn::parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let attr_args: Vec<Meta> = attr_args.into_iter().collect();
    if is_aggregatable(&attr_args) {
        expand_aggr_co_class(&input, &attr_args)
    } else {
        expand_co_class(&input, &attr_args)
    }
}
fn is_aggregatable(attr_args: &[Meta]) -> bool {
    attr_args.iter().any(|arg| match arg {
        Meta::Path(path) => {
            let segments = &path.segments;
            segments.len() == 1
                && segments.first().expect("Invalid attribute syntax").ident == "aggregatable"
        }
        _ => false,
    })
}

#[proc_macro]
pub fn declare_offsets(_: TokenStream) -> TokenStream {
    let it = (0..64usize).map(|n| {
        let ident = Ident::new(&format!("Offset{}", n), Span::call_site());
        quote! {
            #[allow(missing_docs)]
            pub struct #ident;
            impl crate::offset::Offset for #ident {
                const VALUE: usize = #n;
            }
        }
    });

    let out = quote! {
        #(#it)*
    };
    out.into()
}
