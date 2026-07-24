use pdl_compiler::{analyzer, ast::SourceDatabase, backends, parser};

fn main() {
    println!("cargo:rerun-if-changed=pdl/link.pdl");
    let mut sources = SourceDatabase::new();
    let file = parser::parse_file(&mut sources, "pdl/link.pdl").expect("PDL parse error");
    let analyzed = analyzer::analyze(&file).expect("PDL analysis error");
    let code = backends::rust::generate(&sources, &analyzed, &[]);
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("link_gen.rs");
    std::fs::write(out, code).expect("write generated codec");
}
