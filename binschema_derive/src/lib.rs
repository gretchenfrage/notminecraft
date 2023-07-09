
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{
    token::Comma,
    punctuated::Punctuated,
    parse_macro_input,
    DeriveInput,
    Data,
    DataStruct,
    Fields,
    FieldsNamed,
    FieldsUnnamed,
    Field,
    DataEnum,
    Meta,
    Lit,
};
use quote::quote;

fn field_schema(field: &Field) -> TokenStream2 {
    if let Some(attr) = field.attrs
        .iter()
        .find(|attr| attr.path.is_ident("schema"))
    {
        // attribute mode
        // the only attribute we currently support is recurse
        let meta = attr.parse_args::<Meta>()
            .expect("attribute failed to parse");
        let meta =
            match meta {
                Meta::NameValue(meta) => meta,
                _ => panic!("attribute must be name/value style"),
            };
        assert!(meta.path.is_ident("recurse"), "unsupported attribute name");
        let n =
            match meta.lit {
                Lit::Int(int) => int,
                _ => panic!("recurse level must be int"),
            };
        quote! {
            recurse(#n)
        }
    } else {
        // not attributes
        let field_ty = &field.ty;
        quote! {
            %<#field_ty as ::binschema::KnownSchema>::schema(stack)
        }
    }
}

fn fields_schema(fields: &Fields) -> TokenStream2 {
    match fields {
        &Fields::Named(FieldsNamed { ref named, .. }) => {
            // struct-like
            let inner = named.iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let inner = field_schema(field);
                    quote! {
                        (#field_name: #inner)
                    }
                })
                .collect::<Punctuated<_, Comma>>();
            quote! {
                %{
                    let stack = stack.with_none_layer();
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
                field_schema(&unnamed[0])
            } else {
                // normal tuple
                let inner = unnamed.iter()
                    .map(field_schema)
                    .map(|field_schema| quote! {
                        (#field_schema)
                    })
                    .collect::<Punctuated<_, Comma>>();
                quote! {
                %{
                    let stack = stack.with_none_layer();
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

#[proc_macro_derive(KnownSchema, attributes(schema))]
pub fn derive_known_schema(input: TokenStream) -> TokenStream {
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
        &Data::Union(_) => panic!("cannot derive KnownSchema on a union"),
    };
    
    quote! {
        impl ::binschema::KnownSchema for #name {
            fn schema(
                parent_stack: ::binschema::RecurseStack,
            ) -> ::binschema::Schema {
                if let Some(s) = parent_stack.parent_recurse::<Self>() {
                    return s;
                }
                let stack = parent_stack.with_type_layer::<Self>();
                ::binschema::schema!(#schema)
            }
        }
    }.into()
}
