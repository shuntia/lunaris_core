use std::env;
use std::env::current_dir;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let plugin_dir = env::var("PLUGIN_DIR").unwrap_or("../plugins".into());

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
    fs::write(
        Path::new(&out_dir).join("linking.rs"),
        link(find_crates(PathBuf::from(&plugin_dir))),
    )
    .unwrap();
}

fn find_crates(dir: PathBuf) -> Vec<String> {
    let mut ret = vec![];
    for i in dir.read_dir().unwrap() {
        for j in i.unwrap().path().read_dir().unwrap() {
            ret.push(
                j.unwrap()
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            )
        }
    }
    ret
}

fn link(v: Vec<String>) -> String {
    let mut builder = String::new();
    for i in v {
        builder.push_str(&format!("use {i} as _;"));
    }
    builder
}
