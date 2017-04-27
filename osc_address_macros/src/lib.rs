#![recursion_limit="128"]
#![feature(conservative_impl_trait)]
extern crate osc_address;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{MacroInput, MetaItem, NestedMetaItem, Ty};

/// Collects all info from #[osc_address(..)] attributes for a given
/// enum variant.
#[derive(Debug)]
struct OscRouteProperties {
    address: OscBranchFmt,
    var_type: VariantType,
}

/// Describes how to format the portion of the OSC address between adjacent
/// pairs of '/'
#[derive(Debug)]
enum OscBranchFmt {
    /// This branch of the OSC address is a literal string,
    /// e.g. "world" in "/hello/world"
    Str(String),
}

/// In general, each enum variant of the OscAddress type holds both path arguments and data
/// arguments. But the data arguments might be parsed into another OscAddress sub-type.
#[derive(Debug)]
enum VariantType {
    /// SubPath((path_args), (data_args))
    /// Also encompasses units as 0-length tuples.
    SeqSeq,
    /// SubPath((path_args), SubType)
    SeqStruct,
}

#[proc_macro_derive(OscAddress, attributes(osc_address))]
pub fn derive_osc_address(input: TokenStream) -> TokenStream {
    // Parse the string representation into a syntax tree
    let ast = syn::parse_derive_input(&input.to_string()).unwrap();

    // Build the output
    let expanded = impl_osc_address(&ast);

    // Return the generated impl as a TokenStream
    let ts = expanded.parse().unwrap();
    println!("{}", ts);
    ts
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
                let variant_address = "/".to_string() + &variant_address;
                match variant_props.var_type {
                    // Payload IS the message data; not a nested OscAddress
                    VariantType::SeqSeq => quote! {
                        #typename::#variant_ident(_path_args, _msg_data) => {
                            address.push_str(#variant_address);
                        }
                    },
                    // Payload is a nested OscAddress
                    VariantType::SeqStruct => quote! {
                        #typename::#variant_ident(_path_args, ref msg_data) => {
                            address.push_str(#variant_address);
                            msg_data.build_address(address);
                        }
                    },
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
        }
    }
}



/// Return all the configuration data associated with a given enum variant.
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
    let var_type = match variant.data {
        syn::VariantData::Tuple(ref fields) => {
            if fields.len() != 2 {
                panic!("Expected OscAddress enum variant tuple to have exactly two entries: one for path arguments and one for the message payload. Got: {:?}", fields);
            }
            match fields[1].ty {
                // Is the message data a sequence type, or a nested OscAddress?
                Ty::Slice(_) | Ty::Array(_, _) | Ty::Tup(_) => VariantType::SeqSeq,
                _ => VariantType::SeqStruct,
            }
        },
        _ => panic!("Expected OscAddress enum variant to be a tuple. Got: {:?}", variant.data),
    };
    if addresses.len() != 1 {
        panic!("Expected exactly 1 #[osc_address(address=...)] for each enum variant. Saw: {:?}", addresses);
    }
    let address = addresses.into_iter().next().unwrap();
    OscRouteProperties{ address, var_type }
}

/// Return all NestedMetaItems corresponding to
/// #[osc_address ...] attributes
fn get_osc_meta_items<'a>(variant: &'a syn::Variant) -> impl Iterator<Item=&'a syn::NestedMetaItem> + 'a {
    variant.attrs.iter().flat_map(|attr| match attr.value {
        MetaItem::List(ref name, ref items) if name == "osc_address" => Some(items.iter()),
        _ => None,
    }).flat_map(|attrs| attrs)
}

impl OscBranchFmt {
    fn new(fmt: &syn::Lit) -> Self {
        match *fmt {
            syn::Lit::Str(ref s, ref _style) => OscBranchFmt::Str(s.clone()),
            _ => panic!("Expected a string in #[osc_address(address=...)]; got: {:?}", fmt),
        }
    }
}

