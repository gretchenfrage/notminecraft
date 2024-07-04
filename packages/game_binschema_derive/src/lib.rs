
use proc_macro::TokenStream;
use proc_macro2::{
    TokenStream as TokenStream2,
    Span,
};
use quote::quote;
use syn::{
    punctuated::Punctuated,
    token::Comma,
    *,
};


fn fields_schema(fields: &Fields) -> TokenStream2 {
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            // struct-like
            let inner = named.iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_ty = &field.ty;
                    quote! {
                        (#field_name: %<#field_ty as crate::game_binschema::GameBinschema>::schema(game))
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                %{ // TODO: can we not?
                    ::binschema::schema!(
                        struct { #inner }
                    )
                }
            }
        },
        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
            if unnamed.len() == 0 {
                // 0-tuple... just treat it as unit!
                quote! {
                    unit
                }
            } else if unnamed.len() == 1 {
                // newtype (1-tuple)
                let field_ty = &unnamed[0].ty;
                quote! {
                    %<#field_ty as crate::game_binschema::GameBinschema>::schema(game)
                }
            } else {
                // normal tuple
                let inner = unnamed.iter()
                    .map(|field| {
                        let field_ty = &field.ty;
                        quote! {
                            (%<#field_ty as crate::game_binschema::GameBinschema>::schema(game))
                        }
                    })
                    .collect::<Punctuated<_, Comma>>();
                quote! {
                %{
                    ::binschema::schema!(
                        tuple { #inner }
                    )
                }
            }
            }
        },
        &Fields::Unit => {
            quote! {
                unit
            }
        },
    }
}

// Eg 0 => `_0`
//    1 => `_1`
// ...
fn int_ident(n: usize) -> Ident {
    Ident::new(&format!("_{}", n), Span::call_site())
}

fn stringify(ident: &Ident) -> LitStr {
    LitStr::new(&ident.to_string(), ident.span())
}

// Eg `{ foo: Foo, bar: Bar }` => `{ foo: ref _0, bar: ref _1 }`
//    `(Foo, Bar)`             => `(ref _0, ref _1)`
//    ``                       => ``
fn fields_pattern(fields: &Fields, add_ref: bool) -> TokenStream2 {
    let ref_or_no = if add_ref {
        quote! { ref }
    } else {
        quote! {}
    };
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            // struct-like
            let inner = named.iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_name = field.ident.clone().unwrap();
                    let bind_to = int_ident(i);
                    quote! {
                        #field_name: #ref_or_no #bind_to
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                { #inner }
            }
        }
        &Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
            // tuple-like
            let inner = (0..unnamed.len())
                .map(|i| {
                    let bind_to = int_ident(i);
                    quote! {
                        #ref_or_no #bind_to
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                ( #inner )
            }
        }
        &Fields::Unit => {
            // unit-like
            quote! {}
        }
    }
}

fn encode_fields(fields: &Fields) -> TokenStream2 {
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            // struct-like
            let encode_fields = named.iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_str = stringify(field.ident.as_ref().unwrap());
                    let field_ty = &field.ty;
                    let bind_to = int_ident(i);
                    quote! {
                        encoder.begin_struct_field(#field_str)?;
                        <#field_ty as crate::game_binschema::GameBinschema>::encode(#bind_to, encoder, game)?;
                    }
                })
                .collect::<TokenStream2>();
            quote! {
                encoder.begin_struct()?;
                #encode_fields
                encoder.finish_struct()
            }
        },
        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
            if unnamed.len() == 0 {
                // 0-tuple... just treat it as unit!
                quote! {
                    encoder.encode_unit()
                }
            } else if unnamed.len() == 1 {
                // newtype (1-tuple)
                let field_ty = &unnamed[0].ty;
                quote! {
                    <#field_ty as crate::game_binschema::GameBinschema>::encode(_0, encoder, game)
                }
            } else {
                // normal tuple
                let encode_fields = unnamed.iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let field_ty = &field.ty;
                        let bind_to = int_ident(i);
                        quote! {
                            encoder.begin_tuple_elem()?;
                            <#field_ty as crate::game_binschema::GameBinschema>::encode(#bind_to, encoder, game)?;
                        }
                    })
                    .collect::<TokenStream2>();
                quote! {
                    encoder.begin_tuple()?;
                    #encode_fields
                    encoder.finish_tuple()
                }
            }
        },
        &Fields::Unit => {
            // unit-like
            quote! {
                encoder.encode_unit()
            }
        },
    }
}

fn decode_fields(fields: &Fields) -> TokenStream2 {
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            // struct-like
            let encode_fields = named.iter()
                .enumerate()
                .map(|(i, field)| {
                    let field_str = stringify(field.ident.as_ref().unwrap());
                    let field_ty = &field.ty;
                    let bind_to = int_ident(i);
                    quote! {
                        decoder.begin_struct_field(#field_str)?;
                        let #bind_to = <#field_ty as crate::game_binschema::GameBinschema>::decode(decoder, game)?;
                    }
                })
                .collect::<TokenStream2>();
            quote! {
                decoder.begin_struct()?;
                #encode_fields
                decoder.finish_struct()?;
            }
        },
        Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }) => {
            if unnamed.len() == 0 {
                // 0-tuple... just treat it as unit!
                quote! {
                    decoder.decode_unit()?;
                }
            } else if unnamed.len() == 1 {
                // newtype (1-tuple)
                let field_ty = &unnamed[0].ty;
                quote! {
                    let _0 = <#field_ty as crate::game_binschema::GameBinschema>::decode(decoder, game)?;
                }
            } else {
                // normal tuple
                let decode_fields = unnamed.iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let field_ty = &field.ty;
                        let bind_to = int_ident(i);
                        quote! {
                            decoder.begin_tuple_elem()?;
                            let #bind_to = <#field_ty as crate::game_binschema::GameBinschema>::decode(decoder, game)?;
                        }
                    })
                    .collect::<TokenStream2>();
                quote! {
                    decoder.begin_tuple()?;
                    #decode_fields
                    decoder.finish_tuple()?;
                }
            }
        },
        &Fields::Unit => {
            // unit-like
            quote! {
                decoder.decode_unit()?;
            }
        },
    }
}

#[proc_macro_derive(GameBinschema)]
pub fn derive_game_binschema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let schema = match &input.data {
        &Data::Struct(DataStruct { ref fields, .. }) => fields_schema(fields),
        &Data::Enum(DataEnum { ref variants, .. }) => {
            let inner = variants.iter()
                .map(|variant| {
                    let variant_name = &variant.ident;
                    let inner = fields_schema(&variant.fields);
                    quote! {
                        #variant_name(#inner)
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                enum { #inner }
            }
        },
        &Data::Union(_) => panic!("cannot derive GameBinschema on a union"),
    };

    let encode = match &input.data {
        &Data::Struct(DataStruct { ref fields, .. }) => {
            let fields_pattern = fields_pattern(fields, true);
            let encode_fields = encode_fields(fields);
            quote! {
                let & #name #fields_pattern = self;
                #encode_fields
            }
        }
        &Data::Enum(DataEnum { ref variants, .. }) => {
            let cases = variants.iter()
                .enumerate()
                .map(|(i, variant)| {
                    let variant_name = &variant.ident;
                    let fields_pattern = fields_pattern(&variant.fields, true);
                    let variant_ord = LitInt::new(&i.to_string(), Span::call_site());
                    let variant_name_str = stringify(variant_name);
                    let encode_fields = encode_fields(&variant.fields);
                    quote! {
                        & #name :: #variant_name #fields_pattern => {
                            encoder.begin_enum(#variant_ord, #variant_name_str)?;
                            #encode_fields
                        }
                    }
                })
                .collect::<TokenStream2>();
            quote! {
                match self {
                    #cases
                }
            }
        }
        &Data::Union(_) => panic!("cannot derive GameBinschema on a union")
    };

    let decode = match &input.data {
        &Data::Struct(DataStruct { ref fields, .. }) => {
            let decode_fields = decode_fields(fields);
            let fields_pattern = fields_pattern(fields, false);
            quote! {
                #decode_fields
                Ok(#name #fields_pattern)
            }
        }
        &Data::Enum(DataEnum { ref variants, .. }) => {
            let cases = variants.iter()
                .enumerate()
                .map(|(i, variant)| {
                    let variant_pat = LitInt::new(&i.to_string(), Span::call_site());
                    let variant_name = &variant.ident;
                    let variant_name_str = stringify(variant_name);
                    let fields_pattern = fields_pattern(&variant.fields, false);
                    let decode_fields = decode_fields(&variant.fields);
                    quote! {
                        #variant_pat => {
                            decoder.begin_enum_variant(#variant_name_str)?;
                            #decode_fields
                            #name :: #variant_name #fields_pattern
                        }
                    }
                })
                .collect::<TokenStream2>();
            quote! {
                Ok(match decoder.begin_enum()? {
                    #cases
                    _ => unreachable!("invalid enum ordinal for corresponding schema")
                })
            }
        }
        &Data::Union(_) => panic!("cannot derive GameBinschema on a union")
    };
    
    let mut generics = input.generics.clone();
    let additional_where_predicates = generics
        .type_params()
        .map(|type_param| {
            let ident = &type_param.ident;
            parse_quote! { #ident: crate::game_binschema::GameBinschema }
        })
        .collect::<Vec<WherePredicate>>();
    generics.make_where_clause().predicates.extend(additional_where_predicates);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics crate::game_binschema::GameBinschema for #name #ty_generics #where_clause {
            fn schema(game: &::std::sync::Arc<crate::game_data::GameData>) -> ::binschema::Schema {
                ::binschema::schema!(#schema)
            }
    
            fn encode(&self, encoder: &mut ::binschema::Encoder<Vec<u8>>, game: &::std::sync::Arc<crate::game_data::GameData>) -> ::binschema::error::Result<()> {
                #encode
            }

            fn decode(decoder: &mut ::binschema::Decoder<::std::io::Cursor<&[u8]>>, game: &::std::sync::Arc<crate::game_data::GameData>) -> ::binschema::error::Result<Self> {
                #decode
            }
        }
    }.into()
}
