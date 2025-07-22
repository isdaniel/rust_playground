use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};
use syn::__private::TokenStream2;

fn map_fields<F>(fields: &syn::Fields, func: F) -> TokenStream2
where
    F: Fn(&syn::Field) -> TokenStream2
{
    TokenStream2::from_iter(
        fields.iter().map(func)
    )
}

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let builder_ident = syn::Ident::new(&format!("{}Builder", ident), ident.span());

    if let Data::Struct(data_struct) = input.data {
        let builder_fields = map_fields(&data_struct.fields, |f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! {
                #ident: Option<#ty>,
            }
        });

        let builder_set_methods = map_fields(&data_struct.fields, |f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! {
                pub fn #ident(mut self, #ident: #ty) -> Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        });

        let builder_init_fields = map_fields(&data_struct.fields, |f| {
            let ident = &f.ident;
            quote! {
                #ident: self.#ident.unwrap_or_default(),
            }
        });

        return quote!(
            #[derive(Default)]
            pub struct #builder_ident {
                #builder_fields
            }

            impl #builder_ident {
                #builder_set_methods

                pub fn build(self) -> Result<#ident, ()> {
                    Ok(#ident {
                        #builder_init_fields
                    })
                }
            }

            impl #ident {
                pub fn builder() -> #builder_ident {
                    #builder_ident::default()
                }
            }
        ).into();
    };

    TokenStream::from(quote!())
}


//https://github.com/dtolnay/proc-macro-workshop

/*
/// This is a simple command builder that allows you to construct a command with an executable, arguments.
#[derive(Default)]
pub struct Builder {
    executable: String,
    args: Vec<String>,
    current_dir: Option<String>,
}

impl Builder {
    pub fn executable(mut self, executable: String) -> Self {
        self.executable = executable;
        self
    }

    pub fn arg(mut self, arg: String) -> Self {
        self.args.push(arg);
        self
    }

    pub fn current_dir(mut self, dir: String) -> Self {
        self.current_dir = Some(dir);
        self
    }

    pub fn build(self) -> Result<Command, String> {
        if self.executable.is_empty() {
            return Err("Executable cannot be empty".to_string());
        }
        Ok(Command {
            executable: self.executable,
            args: self.args,
            current_dir: self.current_dir,
        })
    }

}

#[derive(Debug)]
pub struct Command {
    executable: String,
    args: Vec<String>,
    current_dir: Option<String>,
}

impl Command {
    pub fn builder() -> Builder {
        Builder::default()
    }
}

fn main() {
    let command = Command::builder()
        .executable("cargo".to_owned())
        .arg("build".to_owned())
        .arg("--release".to_owned())
        .build()
        .unwrap();

    println!("Executing command: {:?}", command);
}
*/
