extern crate bindgen;
extern crate pkg_config;

mod inner {
    
    extern crate bindgen;
    extern crate pkg_config;

    pub fn main() {
        let mut builder = bindgen::builder();

        if let Some(clang_include_dir) = bindgen::get_include_dir() {
            builder.clang_arg("-I");
            builder.clang_arg(clang_include_dir);
        }

        let pkg = pkg_config::probe_library("GraphicsMagick")
            .expect("Could not find GraphicsMagick");

        let mut header = None;

        for path in pkg.include_paths.iter()
            .filter_map(|p| p.to_str()) {
            
            builder.clang_arg("-I");
            builder.clang_arg(path);

            let api = ::std::path::Path::new(path)
                .join("magick")
                .join("api.h");

            if api.metadata().is_ok() {
                header = Some(api.to_string_lossy().into_owned());
            }
        }

        for lib in pkg.libs {
            builder.link(lib);
        }

        let bindings = builder
            .header(header.expect("No header found"))
            .emit_builtins()
            .generate()
            .unwrap();

        bindings
            .write_to_file("src/ffi.rs")
            .unwrap();
    }
}

fn main() {
     if !::std::fs::metadata("src/ffi.rs").is_ok() { inner::main(); }
}