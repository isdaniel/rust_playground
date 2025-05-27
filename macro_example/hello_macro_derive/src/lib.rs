use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(HelloMacro)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream{
    //Rust code as syntax tree
    let ast = syn::parse(input).unwrap(); //DeriveInput
    impl_hello_macro(&ast)
}

fn impl_hello_macro(ast : &syn::DeriveInput) -> TokenStream{
    let name = &ast.ident;
    let gen_token = quote!{
        impl HelloMacro for #name {
            fn hello_macro(){
                println!("Hello World!! My name is {}!",stringify!(#name));
            }
        }
    };

    gen_token.into()
}