#![feature(conservative_impl_trait)]
extern crate osc_address;
extern crate proc_macro;

#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{MacroInput, MetaItem, NestedMetaItem};

/// Collects all info from #[osc_address(..)] attributes for a given
/// enum variant.
#[derive(Debug)]
struct OscRouteProperties {
    address: OscBranchFmt,
}

/// Describes how to format the portion of the OSC address between adjacent
/// pairs of '/'
#[derive(Debug)]
enum OscBranchFmt {
    /// This branch of the OSC address is a literal string,
    /// e.g. "world" in "/hello/world"
    Str(String),
}

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
                let variant_props = get_variant_props(variant);
                let OscBranchFmt::Str(variant_address) = variant_props.address;
                quote! {
                    #typename::#variant_ident => {
                        address.push_str(#variant_address);
                    }
                }
            });

            quote! {
                match *self {
                    #(#arms)*
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


/// Return all NestedMetaItems corresponding to
/// #[osc_address ...] attributes
fn get_osc_meta_items<'a>(variant: &'a syn::Variant) -> impl Iterator<Item=&'a syn::NestedMetaItem> + 'a {
    variant.attrs.iter().flat_map(|attr| match attr.value {
        MetaItem::List(ref name, ref items) if name == "osc_address" => Some(items.iter()),
        _ => None,
    }).flat_map(|attrs| attrs)
}


fn get_variant_props(variant: &syn::Variant) -> OscRouteProperties {
    let mut addresses = Vec::new();
    // Iter all X in #[osc_address X]
    for item in get_osc_meta_items(variant) {
        match *item {
            NestedMetaItem::MetaItem(ref item) => match *item {
                MetaItem::NameValue(ref name, ref lit) => if name == "address" {
                    addresses.push(OscBranchFmt::new(lit));
                },
                _ => panic!("Unsupported #[osc_address] directive: {:?}", item),
            },
            _ => panic!("Unsupported #[osc_address] directive: {:?}", item),
        }
    }
    if addresses.len() != 1 {
        panic!("Expected exactly 1 #[osc_address(address=...)] for each enum variant. Saw: {:?}", addresses);
    }
    OscRouteProperties {
        address: addresses.into_iter().next().unwrap()
    }
}

impl OscBranchFmt {
    fn new(fmt: &syn::Lit) -> Self {
        match *fmt {
            syn::Lit::Str(ref s, ref _style) => OscBranchFmt::Str(s.clone()),
            _ => panic!("Expected a string in #[osc_address(address=...)]; got: {:?}", fmt),
        }
    }
}

