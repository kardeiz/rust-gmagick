extern crate bindgen;

fn main() {

  let gm_path = std::env::var("GM_PATH")
    .ok()
    .unwrap_or_else(|| "/usr/include/GraphicsMagick".to_string());

  let mut builder = bindgen::builder();

  if let Some(clang_include_dir) = bindgen::get_include_dir() {
    builder.clang_arg("-I");
    builder.clang_arg(clang_include_dir);
  }

  let bindings = builder
    .clang_arg("-I")
    .clang_arg(gm_path.as_ref())
    .header("src/gen.h")
    .link("GraphicsMagick")
    .emit_builtins()
    .generate()
    .unwrap();

  bindings
    .write_to_file("src/ffi.rs")
    .unwrap();

}