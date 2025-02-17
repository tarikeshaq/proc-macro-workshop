use proc_macro::TokenStream;

mod expand;
mod parse;
struct Seq {
    content: proc_macro2::TokenStream,
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let seq = syn::parse_macro_input!(input as Seq);
    proc_macro::TokenStream::from(match seq.expand() {
        Ok(res) => res,
        Err(e) => e.to_compile_error(),
    })
}
