use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::{error::Error, path::Path};

use pelite::{util::CStr, FileMap, PeFile};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use search_path::SearchPath;
use vergen::EmitBuilder;

fn get_libpath() -> PathBuf {
    let libname = std::fs::read_to_string("libname.cfg").unwrap();
    let libname = libname.trim();

    let libname = libname.strip_suffix(".dll").unwrap_or(libname);
    let libname = format!("{libname}.dll");
    let path = Path::new(&libname);

    let search_path = SearchPath::new("Path").unwrap();
    let res = search_path.find_file(path);

    if let Some(res) = res {
        return res;
    }

    // search in same directory now
    if !path.exists() {
        panic!("{libname}.dll not found")
    }

    path.to_path_buf()
}

fn main() -> Result<(), Box<dyn Error>> {
    if !cfg!(target_os = "windows") {
        panic!("Only windows OS is supported");
    }

    // winres
    winres::WindowsResource::new().compile()?;

    // vergen
    EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .emit()?;

    //
    // Dll proxy function generation
    //

    let libpath = get_libpath();
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    // read exports from the DLL file

    let mapping = FileMap::open(&libpath)?;
    let pe_file = PeFile::from_bytes(&mapping)?;
    let exports = pe_file.exports()?;
    let exports_by = exports.by()?;

    let mut export_names = Vec::<Option<&CStr>>::new();
    export_names.resize(exports_by.functions().len(), None);
    for (name, index) in exports_by.iter_name_indices() {
        export_names[index] = Some(name?);
    }

    let ordinal_base = exports.ordinal_base();

    // generate func_defs.rs

    let mut func_def = TokenStream::new();

    let export_len = export_names.len();
    func_def.extend(quote! {
        pub static ORDINAL_BASE: u16 = #ordinal_base;
        pub const NUM_FUNCTIONS: usize = #export_len;
    });

    let mut array_items = TokenStream::new();
    for export_name in export_names.iter() {
        if let Some(name) = export_name {
            assert!(!name.contains(&b'"') && !name.contains(&b'\\'));
            let text = format!("{name}\0").to_token_stream();

            array_items.extend(quote! {
                Some(#text.as_bytes()),
            });
        } else {
            array_items.extend(quote! {
                None,
            });
        }
    }

    func_def.extend(quote! {
        pub static FUNCTION_NAMES: [Option<&'static [u8]>; NUM_FUNCTIONS] = [#array_items];
    });

    // generate proxy_funcs.rs and exports.def

    let mut exports_def = File::create(out_dir.join("Exports.def"))?;
    writeln!(&mut exports_def, "LIBRARY {}\n", env!("CARGO_PKG_NAME"))?;
    writeln!(&mut exports_def, "EXPORTS")?;

    let mut export_asm = Vec::new();
    for (i, export_name) in export_names.iter().enumerate() {
        if let Some(name) = export_name {
            let asm = generate_asm(name.to_str()?, i);
            export_asm.push(asm);

            writeln!(&mut exports_def, "\t{name} @{}", i + ordinal_base as usize)?;
        } else {
            let asm = generate_asm(&format!("unk{i}"), i);
            export_asm.push(asm);

            writeln!(
                &mut exports_def,
                "\tunk{i} @{} NONAME PRIVATE",
                i + ordinal_base as usize
            )?;
        }
    }

    let proxy_funcs = quote! {
        mod _asm {
            use std::arch::global_asm;

            #(#export_asm)*
        }
    };

    func_def.extend(proxy_funcs);
    fs::write(out_dir.join("func_defs.rs"), func_def.to_string())?;
    exports_def.flush()?;

    // write compiler flags

    println!(
        "cargo:rustc-cdylib-link-arg=/def:{}",
        out_dir.join("Exports.def").to_str().unwrap()
    );

    Ok(())
}

fn generate_asm(name: &str, index: usize) -> TokenStream {
    let index = std::mem::size_of::<*const ()>() * index;

    let asm = format!(
        "
            .global {name}
            {name}:
                jmp [rip + {{base}} + {index}]
        "
    );

    quote! {
        global_asm!(
            #asm,
            base = sym super::FUNCTION_PTRS
        );
    }
}
