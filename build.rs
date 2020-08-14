fn main() {

    pkg_config::probe_library("GraphicsMagick").expect("Could not find GraphicsMagick");

    let out_path = std::path::Path::new("src/ffi.rs");

    if out_path.exists() { return; }

    let bindings = bindgen::Builder::default()
        .derive_debug(true)
        .impl_debug(true)
        .default_enum_style(bindgen::EnumVariation::Rust { non_exhaustive: false })
        .header_contents("wrapper.h", "#include \"magick/api.h\"")
        .generate()
        .unwrap();

    bindings.write_to_file(out_path).unwrap();

}
