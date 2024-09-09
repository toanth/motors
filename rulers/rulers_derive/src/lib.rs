use proc_macro::TokenStream;
use quote::quote;
use rulers::command::UgiCommandTrait;
use std::fmt::{Debug, Display};
use std::str::FromStr;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Field, Type};

fn add(left: u64, right: u64) -> u64 {
    left + right
}

// #[proc_macro_attribute]
// pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let ast = syn::parse_macro_input!(input as Attribute);
//     let name = &ast.ident;
//     let gen = quote! {
//         impl UgiCommand for $name {
//
//         }
//     };
//     gen.into()
// }

trait UgiCommandOption: Display + Debug + FromStr {}

#[proc_macro_derive(UgiCommand)]
pub fn command_macro(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let mut write_ugi = quote! {};
    match ast.data {
        Data::Struct(data) => {
            for field in data.fields {
                let ident = field
                    .ident
                    .expect("You need to use named members to derive UgiCommand");
                write_ugi = quote! {
                    $write_ugi
                    write!(f, "{} ", $ident)?;
                }
            }
        }
        Data::Enum(_) => {}
        Data::Union(_) => {
            unreachable!("Don't put this macro on an union, only enums and structs are supported")
        }
    }
    let gen = quote! {
        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                $write_ugi
            }
        }
      impl UgiCommand for $name {
            fn name(&self) -> self.to_string()
        }
    };
    gen.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
