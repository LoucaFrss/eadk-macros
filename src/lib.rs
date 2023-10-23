use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, FnArg, ItemFn, Signature, Type,
    TypePath, TypeSlice,
};

#[proc_macro_attribute]
pub fn main(_args: TokenStream, func: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(func as ItemFn);

    if let Err(err) = verify_sig(&mut func.sig).map_err(|err| err.into_compile_error()) {
        return err.into();
    }

    let data = std::fs::read_to_string(Path::new("config.toml")).unwrap();
    let App { config } = toml::from_str(&data).unwrap();

    let name = format!("{}\0", config.name);
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len();

    let api_level = config.api_level;
    println!("{:?}", std::env::current_dir().unwrap().as_path());
    let icon_len = File::open("./target/icon.nwi")
        .unwrap()
        .metadata()
        .unwrap()
        .len() as usize;

    let tokens = quote! {
        use eadk::prelude::*;
        #[used]
        #[link_section = ".rodata.eadk_app_name"]
        pub static EADK_APP_NAME: [u8; #name_len] = [#(#name_bytes),*];

        #[used]
        #[link_section = ".rodata.eadk_api_level"]
        pub static EADK_APP_API_LEVEL: u32 = #api_level;

        #[used]
        #[link_section = ".rodata.eadk_app_icon"]
        pub static EADK_APP_ICON: [u8; #icon_len] = *include_bytes!("../target/icon.nwi");
        #[no_mangle]
        #func

    #[cfg(debug_assertions)]
    #[panic_handler]
    fn panic(panic: &core::panic::PanicInfo<'_>) -> ! {
        unsafe {
            eadk::display::draw_string(
                panic.location().unwrap_unchecked().file().as_bytes(),
                eadk::Point { x: 0, y: 40 },
                false,
                eadk::rgb!(255, 0, 0),
                eadk::rgb!(255, 255, 255),
            );
            eadk::display::draw_string(
                panic
                    .message()
                    .unwrap_unchecked()
                    .as_str()
                    .unwrap_unchecked()
                    .as_bytes(),
                eadk::Point { x: 0, y: 0 },
                false,
                eadk::rgb!(255, 0, 0),
                eadk::rgb!(255, 255, 255),
            );
            eadk::eprintln!("\n\nline {}.", panic.location().unwrap().line());
        }

        loop {} // FIXME: Do something better. Exit the app maybe?
    }

    #[cfg(not(debug_assertions))]
    #[panic_handler]
    fn panic(_panic: &PanicInfo<'_>) -> ! {
        loop {}
    }


        };

    tokens.into()
}

#[derive(Deserialize)]
struct App {
    pub config: Config,
}
#[derive(Deserialize)]

struct Config {
    pub name: String,
    pub icon: PathBuf,
    pub api_level: u32,
    pub external_data: Option<PathBuf>,
}

fn is_u8_slice(ty: Type) -> bool {
    if let Type::Reference(reference) = ty {
        if let Type::Slice(TypeSlice { elem, .. }) = &*reference.elem {
            if let Type::Path(TypePath { path, .. }) = *elem.clone() {
                if let Some(segment) = path.segments.first() {
                    if segment.ident.to_string() == "u8" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn verify_sig(sig: &mut Signature) -> Result<(), syn::Error> {
    if sig.inputs.len() != 1 {
        return Err(syn::Error::new(
            sig.span().into(),
            format!(
                "Expected only 1 argument to main function, found {}",
                sig.inputs.len()
            ),
        ));
    }
    if let FnArg::Typed(ty) = sig.inputs.first().ok_or(syn::Error::new(
        sig.span().into(),
        "Expected 1 argument to main function",
    ))? {
        return match is_u8_slice(*ty.ty.clone()) {
            true => {
                sig.inputs = Punctuated::new();
                Ok(())
            }
            false => Err(syn::Error::new(
                sig.span().into(),
                "Invalid function signature, expected argument of type &[u8]",
            )),
        };
    }

    Err(syn::Error::new(
        sig.span().into(),
        "Invalid main function signature",
    ))
}
