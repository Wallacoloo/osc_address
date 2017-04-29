// quote/syn crates require high macro expandion recursion limits
#![recursion_limit="128"]
#![feature(conservative_impl_trait)]
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
    path_args_type: PathArgsType,
    msg_args_type: MsgArgsType,
}

/// Describes how to format the portion of the OSC address between adjacent
/// pairs of '/'
#[derive(Debug)]
enum OscBranchFmt {
    /// This branch of the OSC address is a literal string,
    /// e.g. "world" in "/hello/world"
    Str(String),
    /// No format string was provided. Presumably there is a path argument
    /// and it implements FromStr/ToString.
    None,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum PathArgsType {
    /// No path arguments (aka 'unit', ())
    Unit,
    /// There is a path argument.
    One,
}

#[derive(Debug)]
enum MsgArgsType {
    /// SubPath(<path_args>, (data_args))
    /// Also encompasses units as 0-length tuples.
    Seq,
    /// SubPath(<path_args>, SubType)
    /// Presumably the SubType also implements OscMessage.
    Struct,
}

#[proc_macro_derive(OscMessage, attributes(osc_address))]
pub fn derive_osc_address(input: TokenStream) -> TokenStream {
    // Parse the string representation into a syntax tree
    let ast = syn::parse_derive_input(&input.to_string()).unwrap();

    // Build the output
    let expanded = impl_osc_address(&ast);

    // Return the generated impl as a TokenStream
    let ts = expanded.parse().unwrap();
    //println!("{}", ts);
    ts
}

fn impl_osc_address(ast: &MacroInput) -> quote::Tokens {
    let typename = &ast.ident;
    // match the element the #[derive(OscMessage)] statement is applied to,
    // e.g. "enum { ... }" in
    // #[derive(OscMessage)]
    // enum MyEnum { X, Y, Z(u3) }
    let build_address_impl = match ast.body {
        syn::Body::Enum(ref variants) => {
            // variants encodes the name, tuple data, and any #[osc_address(...)]
            // attrs applied to each variant of the enum
            let arms = variants.iter().map(|variant| {
                let variant_ident = variant.ident.clone();
                let variant_props = get_variant_props(variant);
                let address_push_impl = match variant_props.address {
                    // This component of the address is a string constant;
                    // push the string to the address being built.
                    OscBranchFmt::Str(variant_address) => {
                        let variant_address = "/".to_string() + &variant_address;
                        quote!{
                            address.push_str(#variant_address);
                        }
                    },
                    // This component of the address is a variable;
                    // write that variable to the address being built.
                    OscBranchFmt::None => quote! {
                        address.push_str(&format!("/{}", path_arg));
                    },
                };
                let recurse_build_impl = match variant_props.msg_args_type {
                    // Payload IS the message data; not a nested OscMessage
                    MsgArgsType::Seq => quote! {},
                    MsgArgsType::Struct => quote! {
                        OscMessage::build_address(msg_data, address);
                    },
                };
                // Create the variant match case that pushes the component name
                // and then builds the remainder of the address.
                quote! {
                    #typename::#variant_ident(ref path_arg, ref msg_data) => {
                        #address_push_impl
                        #recurse_build_impl
                    },
                }
            });

            quote! {
                match *self {
                    #(#arms)*
                }
            }
        },
        // #[derive(OscMessage)] on a Struct is used to treat that struct as a
        // message payload; therefore it HAS no address.
        syn::Body::Struct(ref _variant_data) => {
            quote! { }
        }
    };
    let (do_impl_serde, serialize_body_impl) = match ast.body {
        syn::Body::Enum(ref variants) => (true, {
            // variants encodes the name, tuple data, and any #[osc_address(...)]
            // attrs applied to each variant of the enum
            let arms = variants.iter().map(|variant| {
                let variant_ident = variant.ident.clone();
                let variant_props = get_variant_props(variant);
                match variant_props.msg_args_type {
                    // Payload IS the message data; not a nested OscMessage
                    MsgArgsType::Seq => quote! {
                        #typename::#variant_ident(ref _path_args, ref msg_data) => {
                            serde::ser::SerializeTuple::serialize_element(serializer, msg_data)
                        }
                    },
                    // Payload is a nested OscMessage
                    MsgArgsType::Struct => quote! {
                        #typename::#variant_ident(ref _path_args, ref msg_data) => {
                            OscMessage::serialize_body(msg_data, serializer)
                        }
                    },
                }
            });

            quote! {
                match *self {
                    #(#arms)*
                }
            }
        }),
        // #[derive(OscMessage)] on a Struct is used to treat that struct as a
        // message payload; therefore, the user should implemente serde::Serialize
        // on their own (perhaps with #[derive(Serialize)]), and we relay to that
        syn::Body::Struct(ref _variant_data) => (false, quote! {
            serde::ser::SerializeTuple::serialize_element(serializer, self)
        })
    };

    let deserialize_body_impl = match ast.body {
        syn::Body::Enum(ref variants) => {
            // create a series of:
            // if address == "<variant_address>" {
            //     return Ok(#typename::#variant_ident((), seq.next_element()?.unwrap()))
            // }
            // // ...
            let arms = variants.iter().map(|variant| {
                let variant_ident = variant.ident.clone();
                let variant_props = get_variant_props(variant);
                match variant_props.msg_args_type {
                    // Payload IS the message data; not a nested OscMessage
                    // By necessity this is the leaf message, so we we don't need
                    // to split the component name off of the address.
                    MsgArgsType::Seq => match variant_props.address {
                        OscBranchFmt::Str(component_name) => quote! {
                            if component_name == #component_name && downstream_address.is_empty() {
                                return Ok(#typename::#variant_ident((), seq.next_element()?.unwrap()));
                            }
                        },
                        OscBranchFmt::None => quote! {
                            // if we can parse the path argument, then the address variant is matched
                            if let Ok(path_arg) = component_name.parse() {
                                return Ok(#typename::#variant_ident(path_arg, seq.next_element()?.unwrap()));
                            }
                        },
                    },
                    // Payload is a nested OscMessage
                    MsgArgsType::Struct => match variant_props.address {
                        OscBranchFmt::Str(component_name) => quote! {
                            if component_name == #component_name {
                                return Ok(#typename::#variant_ident((), OscMessage::deserialize_body(downstream_address, seq)?));
                            }
                        },
                        OscBranchFmt::None => quote! {
                            // if we can parse the path argument, then the address variant is matched
                            if let Ok(path_arg) = component_name.parse() {
                                return Ok(#typename::#variant_ident(path_arg, OscMessage::deserialize_body(downstream_address, seq)?));
                            }
                        },
                    }
                }
            });
            quote! {
                // split the address at the next "/":
                // start from idx=1 because the address begins with "/<component_name>/<downstream ...>"
                let slash_idx = address[1..].find('/');
                let downstream_address = match slash_idx {
                    None => String::new(),
                    Some(idx) => address.split_off(1+idx),
                };
                let component_name = &address[1..];
                #(#arms)*
                // If no patterns matched, then:
                return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(component_name), &"an OSC component name that matches one of the enum variants"));
            }
        },
        syn::Body::Struct(ref _variant_data) => quote! {
            if address != "" && address != "/" {
                return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(&address), &"the OSC path to be terminated by this point"));
            }
            let me = seq.next_element()?;
            match me {
                None => Err(serde::de::Error::invalid_length(1, &"a sequence representing an OSC message payload")),
                Some(me) => Ok(me)
            }
        },
    };


    let serialize_impl = if do_impl_serde {
        quote! {
            impl serde::Serialize for #typename {
                fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    // serialization is a two-step process.
                    // 1: Serialize the message address.
                    // 2: Serialize the ACTUAL message payload; we may have to
                    //    recurse through multiple OscMessage instances to
                    //    locate the leaf payload.
                    let mut tup = serializer.serialize_tuple(2)?;
                    serde::ser::SerializeTuple::serialize_element(&mut tup, &OscMessage::get_address(self))?;
                    // Now serialize the message payload
                    OscMessage::serialize_body(self, &mut tup)?;
                    serde::ser::SerializeTuple::end(tup)
                }
            }
        }
    } else {
        quote! {}
    };

    let deserialize_impl = if do_impl_serde {
        quote! {
            impl<'de> serde::Deserialize<'de> for #typename {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where D: serde::Deserializer<'de>
                {
                    deserializer.deserialize_seq(ToplevelVisitor)
                }
            }
            struct ToplevelVisitor;
            impl<'de> serde::de::Visitor<'de> for ToplevelVisitor {
                type Value = #typename;
                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("a tuple of (String, (msg_args ...))")
                }
                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where A: serde::de::SeqAccess<'de>
                {
                    let address: Option<String> = seq.next_element()?;
                    let address = match address {
                        None => Err(serde::de::Error::invalid_length(0, &"an OSC address string, followed by a sequence of message arguments")),
                        Some(addr) => Ok(addr),
                    }?;
                    #typename::deserialize_body(address, seq)
                }
            }
        }
    } else {
        quote! {}
    };


    let dummy_const = syn::Ident::new(format!("_IMPL_OSCADDRESS_FOR_{}", typename));
    quote! {
        // Effectively namespace the OscMessage macro implementations
        // to prevent imports from polluting user's namespace
        #[allow(non_upper_case_globals)]
        const #dummy_const: () = {
            extern crate serde;
            impl<'de> OscMessage<'de> for #typename {
                fn build_address(&self, address: &mut String) {
                    #build_address_impl
                }
                fn serialize_body<S: serde::ser::SerializeTuple>(&self, serializer: &mut S) -> Result<(), S::Error> {
                    #serialize_body_impl
                }
                fn deserialize_body<D: serde::de::SeqAccess<'de>>(mut address: String, mut seq: D) -> Result<#typename, D::Error> {
                    #deserialize_body_impl
                }
            }
            #serialize_impl
            #deserialize_impl
        };
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
    let (path_args_type, msg_args_type) = match variant.data {
        syn::VariantData::Tuple(ref fields) => {
            if fields.len() != 2 {
                panic!("Expected OscMessage enum variant tuple to have exactly two entries: one for path arguments and one for the message payload. Got: {:?}", fields);
            }
            let path_args_type = match fields[0].ty {
                Ty::Tup(ref v) if v.len() == 0 => PathArgsType::Unit,
                _ => PathArgsType::One,
            };
            let msg_args_type = match fields[1].ty {
                // Is the message data a sequence type, or a nested OscMessage?
                Ty::Slice(_) | Ty::Array(_, _) | Ty::Tup(_) => MsgArgsType::Seq,
                _ => MsgArgsType::Struct,
            };
            (path_args_type, msg_args_type)
        },
        _ => panic!("Expected OscMessage enum variant to be a tuple. Got: {:?}", variant.data),
    };
    // Decode the address
    let address = if addresses.len() > 1 {
        panic!("Expected no more than one #[osc_address(address=...)] for each enum variant. Saw: {:?}", addresses);
    } else if addresses.len() == 1 {
        addresses.into_iter().next().unwrap()
    } else {
        OscBranchFmt::None
    };
    // Verify illegal attribute combinations
    if let OscBranchFmt::Str(_) = address {
        if path_args_type != PathArgsType::Unit {
            panic!("A #[osc_address(address=\"<literal>\")] directive implies no path arguments, but both were found");
        }
    }
    OscRouteProperties{ address, path_args_type, msg_args_type }
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

