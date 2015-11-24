extern crate bindgen;
extern crate pkg_config;

fn main() {

  let mut builder = bindgen::builder();

  if let Some(clang_include_dir) = bindgen::get_include_dir() {
    builder.clang_arg("-I");
    builder.clang_arg(clang_include_dir);
  }

  let pkg = pkg_config::find_library("GraphicsMagick").unwrap();

  for path in pkg.include_paths.iter().filter_map(|p| p.to_str()) {
    builder.clang_arg("-I");
    builder.clang_arg(path);
  }

  for lib in pkg.libs {
    builder.link(lib);
  }

  let bindings = builder
    .header("src/gen.h")
    .emit_builtins()
    .generate()
    .unwrap();

  bindings
    .write_to_file("src/ffi.rs")
    .unwrap();

}