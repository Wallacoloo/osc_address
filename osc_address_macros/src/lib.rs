#![feature(conservative_impl_trait)]
extern crate osc_address;
extern crate proc_macro;

#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{MacroInput, MetaItem, NestedMetaItem};

#[proc_macro_derive(OscAddress, attributes(osc_address))]
pub fn derive_osc_address(input: TokenStream) -> TokenStream {
    // Parse the string representation into a syntax tree
    let ast = syn::parse_derive_input(&input.to_string()).unwrap();

    // Build the output
    let expanded = impl_osc_address(&ast);

    // Return the generated impl as a TokenStream
    expanded.parse().unwrap()
}

fn impl_osc_address(ast: &MacroInput) -> quote::Tokens {
    let typename = &ast.ident;
    // match the element the #[derive(OscAddress)] statement is applied to,
    // e.g. "enum { ... }" in
    // #[derive(OscAddress)]
    // enum MyEnum { X, Y, Z(u3) }
    let build_address_impl = match ast.body {
        syn::Body::Enum(ref variants) => {
            // variants encodes the name, tuple data, and any #[osc_address(...)]
            // attrs applied to each variant of the enum
            let arms = variants.iter().map(|variant| {
                let variant_ident = variant.ident.clone();
                let variant_address = get_variant_address(variant).unwrap();
                quote! {
                    #typename::#variant_ident => {
                        address.push_str(#variant_address);
                    }
                }
            });

            quote! {
                match *self {
                    #(#arms)*
                    //"test".to_string()
                }
            }
        },
        syn::Body::Struct(ref _variant_data) => {
            quote! { }
        }
    };

    quote! {
        impl OscAddress for #typename {
            fn build_address(&self, address: &mut String) {
                #build_address_impl
            }
            fn get_address(&self) -> String {
                let mut s = String::new();
                self.build_address(&mut s);
                s
            }
        }
    }
}

/// Within an enum like:
/// #[derive(OscAddress)]
/// enum MyMsg {
///     #[osc_address(address = "/opta")]
///     OptionA(..),
///     #[osc_address(address = "/optb")]
///     OptionB(..),
/// }
/// this function will return "/opta" or "/optb", based on the Variant passed.
fn get_variant_address(variant: &syn::Variant) -> Option<&String> {
    // TODO: make sure there's only one address!
    matching_osc_meta_strs(variant, "address").next()
}

/// Return all NestedMetaItems corresponding to
/// #[osc_address ...] attributes
fn get_osc_meta_items<'a>(variant: &'a syn::Variant) -> impl Iterator<Item=&'a syn::NestedMetaItem> + 'a {
    variant.attrs.iter().flat_map(|attr| match attr.value {
        MetaItem::List(ref name, ref items) if name == "osc_address" => Some(items.iter()),
        _ => None,
    }).flat_map(|attrs| attrs)
}

/// Return all values of
/// #[osc_address(attr_name=<value>)]
/// <value> could be Lit::Str, Lit::Int, etc.
fn matching_osc_meta_vals<'a>(variant: &'a syn::Variant, attr_name: &'a str) -> impl Iterator<Item=&'a syn::Lit> + 'a {
    get_osc_meta_items(variant).filter_map(move |nest_item| match *nest_item {
        NestedMetaItem::MetaItem(ref item) => match *item {
            MetaItem::NameValue(ref name, ref lit) if name == attr_name => Some(lit),
            _ => None,
        },
        _ => None
    })
}

/// Return all values of
/// #[osc_address(attr_name=<value>)]
/// where <value> is a string.
fn matching_osc_meta_strs<'a>(variant: &'a syn::Variant, attr_name: &'a str) -> impl Iterator<Item=&'a String> + 'a {
    matching_osc_meta_vals(variant, attr_name).filter_map(|lit| {
        match *lit {
            syn::Lit::Str(ref s, ref _style) => Some(s),
            // TODO: error on not-a-string.
            _ => None,
        }
    })
}
