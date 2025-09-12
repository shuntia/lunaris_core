use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    let minor = env::var("CARGO_PKG_VERSION_MINOR").unwrap();
    let patch = env::var("CARGO_PKG_VERSION_PATCH").unwrap();
    let pre = env::var("CARGO_PKG_VERSION_PRE").ok();

    let full = env::var("CARGO_PKG_VERSION").unwrap();

    let contents = format!(
        r#"
        pub const VERSION_FULL: &str = "{full}";
        pub const VERSION_MAJOR: u32 = {major};
        pub const VERSION_MINOR: u32 = {minor};
        pub const VERSION_PATCH: u32 = {patch};
        pub const VERSION_PRE: Option<&'static str> = {};
        "#,
        match pre {
            Some(p) if !p.is_empty() => format!("Some(\"{p}\")"),
            _ => "None".to_string(),
        }
    );

    fs::write(Path::new(&out_dir).join("version.rs"), contents).unwrap();
}
