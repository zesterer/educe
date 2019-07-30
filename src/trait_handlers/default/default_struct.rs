use std::str::FromStr;
use std::fmt::Write;

use super::super::TraitHandler;
use super::models::TypeAttributeBuilder;
use super::models::FieldAttributeBuilder;

use crate::Trait;
use crate::proc_macro2::TokenStream;
use crate::syn::{DeriveInput, Meta, Data, Lit, Generics};
use crate::quote::ToTokens;

pub struct DefaultStructHandler;

impl TraitHandler for DefaultStructHandler {
    fn trait_meta_handler(ast: &DeriveInput, tokens: &mut TokenStream, traits: &[Trait], meta: &Meta) {
        let type_attribute = TypeAttributeBuilder {
            enable_new: true,
            enable_expression: true,
            enable_bound: true,
        }.from_default_meta(meta);

        let bound = type_attribute.bound.into_punctuated_where_predicates_by_generic_parameters(&ast.generics.params);

        let mut builder_tokens = TokenStream::new();

        if let Data::Struct(data) = &ast.data {
            match type_attribute.expression {
                Some(expression) => {
                    for field in data.fields.iter() {
                        let _ = FieldAttributeBuilder {
                            enable_value: false,
                            enable_expression: false,
                        }.from_attributes(&field.attrs, traits);
                    }

                    builder_tokens.extend(quote!(#expression));
                }
                None => {
                    let mut field_attributes = Vec::new();
                    let mut field_names = Vec::new();
                    let mut types = Vec::new();

                    let is_tuple = {
                        if let Some(field) = data.fields.iter().next() {
                            if let Some(_) = field.ident {
                                false
                            } else {
                                true
                            }
                        } else {
                            true
                        }
                    };

                    for (index, field) in data.fields.iter().enumerate() {
                        let field_attribute = FieldAttributeBuilder {
                            enable_value: true,
                            enable_expression: true,
                        }.from_attributes(&field.attrs, traits);

                        let field_name = if let Some(ident) = field.ident.as_ref() {
                            ident.to_string()
                        } else {
                            format!("{}", index)
                        };

                        field_attributes.push(field_attribute);
                        field_names.push(field_name);
                        types.push(field.ty.clone());
                    }

                    if field_names.is_empty() {
                        let ident = &ast.ident;

                        builder_tokens.extend(quote!(#ident));
                    } else {
                        let ident = ast.ident.to_string();

                        let mut struct_tokens = if is_tuple {
                            format!("{ident}(", ident = ident)
                        } else {
                            format!("{ident} {{ ", ident = ident)
                        };

                        for (index, field_attribute) in field_attributes.into_iter().enumerate() {
                            let field_name = &field_names[index];
                            let typ = &types[index];

                            if !is_tuple {
                                struct_tokens.write_fmt(format_args!("{field_name}: ", field_name = field_name)).unwrap();
                            }

                            match field_attribute.value {
                                Some(value) => {
                                    match &value {
                                        Lit::Str(s) => {
                                            struct_tokens.write_fmt(format_args!("core::convert::Into::into({s})", s = s.into_token_stream().to_string())).unwrap();
                                        }
                                        _ => {
                                            struct_tokens.push_str(&value.into_token_stream().to_string());
                                        }
                                    }
                                }
                                None => match field_attribute.expression {
                                    Some(expression) => {
                                        struct_tokens.push_str(&expression);
                                    }
                                    None => {
                                        let typ = typ.into_token_stream().to_string();

                                        struct_tokens.write_fmt(format_args!("<{typ} as core::default::Default>::default()", typ = typ)).unwrap();
                                    }
                                }
                            }

                            struct_tokens.push_str(", ");
                        }

                        if is_tuple {
                            struct_tokens.push(')');
                        } else {
                            struct_tokens.push('}');
                        }

                        builder_tokens.extend(TokenStream::from_str(&struct_tokens).unwrap());
                    }
                }
            }
        }

        let ident = &ast.ident;

        let mut generics_cloned: Generics = ast.generics.clone();

        let where_clause = generics_cloned.make_where_clause();

        for where_predicate in bound {
            where_clause.predicates.push(where_predicate);
        }

        let (impl_generics, ty_generics, where_clause) = generics_cloned.split_for_impl();

        let default_impl = quote! {
            impl #impl_generics core::default::Default for #ident #ty_generics #where_clause {
                fn default() -> Self {
                    #builder_tokens
                }
            }
        };

        tokens.extend(default_impl);

        if type_attribute.new {
            let new_impl = quote! {
                impl #impl_generics #ident #ty_generics #where_clause {
                    #[inline]
                    fn new() -> Self {
                        <Self as core::default::Default>::default()
                    }
                }
            };

            tokens.extend(new_impl);
        }
    }
}