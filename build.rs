use cargo_toml::Manifest;
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::env::temp_dir;
use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::path::PathBuf;
use toml::from_str;

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

    let temp_dir = Path::join(&temp_dir(), "lunaris_build");

    let mut plugins = String::new();
    let plugins = if temp_dir.join("plugins.toml").exists() {
        File::open(Path::join(&temp_dir, "plugins.toml"))
            .unwrap()
            .read_to_string(&mut plugins)
            .unwrap();
        from_str(&plugins).unwrap()
    } else {
        panic!("Expected plugins.toml to exist. prepare again.");
    };

    println!(
        "{:?}",
        Path::new(&out_dir)
            .read_dir()
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<DirEntry>>()
    );

    let mut plugin_out = File::create(Path::new(&out_dir).join("plugins.rs")).unwrap();
    let mut plugin_string = gen_enum(&plugins);
    plugin_string.push_str(&gen_impls(&plugins));

    plugin_out.write_all(plugin_string.as_bytes()).unwrap();
}

fn gen_impls(plugins: &PluginCollection) -> String {
    let mut builder: String = "use lunaris_api::plugin::Plugin;\nuse lunaris_api::plugin::PluginContext;\nimpl lunaris_api::plugin::Plugin for PluginEnum{\n".into();

    for i in [
        PluginMethod {
            name: "name",
            return_type: "&'static str",
            args: vec![("&self", "")],
        },
        PluginMethod {
            name: "init",
            return_type: "",
            args: vec![("&self", ""), ("ctx", "PluginContext")],
        },
        PluginMethod {
            name: "update_world",
            return_type: "",
            args: vec![("&mut self", ""), ("ctx", "PluginContext")],
        },
        PluginMethod {
            name: "report",
            return_type: "lunaris_api::plugin::PluginReport",
            args: vec![("&self", ""), ("ctx", "PluginContext")],
        },
        PluginMethod {
            name: "shutdown",
            return_type: "",
            args: vec![("self", ""), ("ctx", "PluginContext")],
        },
        PluginMethod {
            name: "register_menu",
            return_type: "",
            args: vec![
                ("&self", ""),
                ("menu_bar", "&mut lunaris_api::egui::MenuBar"),
            ],
        },
        PluginMethod {
            name: "reset",
            return_type: "",
            args: vec![("&mut self", ""), ("ctx", "PluginContext")],
        },
    ] {
        builder.push_str(&format!(
            "fn {}({}){}{{\n",
            i.name,
            i.args.iter().fold(String::new(), |mut acc, content| {
                if content.1.is_empty() {
                    acc.push_str(&format!("{},", content.0));
                } else {
                    acc.push_str(&format!("{}:{},", content.0, content.1));
                }
                acc
            }),
            if i.return_type.is_empty() {
                ""
            } else {
                &format!("->{}", i.return_type)
            }
        ));
        builder.push_str("match self{\n");
        for plugin in plugins.iter() {
            builder.push_str(&call_on_inner(plugin, i.name, &i.args))
        }
        builder.push_str("}\n}\n");
    }
    builder.push_str("fn new()->Self{Self::Dummy(dummy::Dummy{})}");
    builder.push_str("}\nimpl lunaris_api::plugin::Gui for PluginEnum{");
    for i in [PluginMethod {
        name: "ui",
        return_type: "",
        args: vec![
            ("&self", ""),
            ("ui", "&mut lunaris_api::egui::Ui"),
            ("ctx", "PluginContext"),
        ],
    }] {
        builder.push_str(&format!(
            "fn {}({}){}{{\n",
            i.name,
            i.args.iter().fold(String::new(), |mut acc, content| {
                if content.1.is_empty() {
                    acc.push_str(&format!("{},", content.0));
                } else {
                    acc.push_str(&format!("{}:{},", content.0, content.1));
                }
                acc
            }),
            if i.return_type.is_empty() {
                ""
            } else {
                &format!("->{}", i.return_type)
            }
        ));
        builder.push_str("match self{\n");
        for plugin in plugins.iter() {
            if plugin.plugin.features.contains(&PluginFeature::Gui) {
                builder.push_str(&call_on_inner(plugin, i.name, &i.args))
            } else {
                builder.push_str(&format!("Self::{}(_)=>{{}},", plugin.plugin.name,))
            }
        }
        builder.push_str("}\n}\n");
    }
    builder.push('}');

    builder.push_str("impl PluginEnum{");
    builder.push_str("pub fn new(variant:&str)->Self{\nmatch variant{\n");
    for plugin in plugins.iter() {
        builder.push_str(&format!(
            "\"{}::{}\"=>Self::{}({}::{}::new()),",
            plugin.cargo.package.as_ref().unwrap().name,
            plugin.plugin.name,
            plugin.plugin.name,
            plugin.cargo.package.as_ref().unwrap().name,
            plugin.plugin.name
        ));
    }
    builder.push_str("x=>panic!(\"Referenced invalid variant: {x}\")");
    builder.push('}');
    builder.push('}');
    builder.push('}');

    builder
}

struct PluginMethod {
    name: &'static str,
    return_type: &'static str,
    args: Vec<(&'static str, &'static str)>,
}

fn call_on_inner(
    plugin: &PluginCrate,
    method: &str,
    args: &Vec<(&'static str, &'static str)>,
) -> String {
    format!(
        "Self::{}(content)=>{}::{}::{}({}),\n",
        plugin.plugin.name,
        plugin.cargo.package.as_ref().unwrap().name,
        plugin.plugin.name,
        method,
        args.iter().fold(String::new(), |acc, val| {
            format!(
                "{acc}{},",
                if val.0.contains("self") {
                    "content"
                } else {
                    val.0
                }
            )
        })
    )
}

fn gen_enum(plugins: &PluginCollection) -> String {
    let mut enum_str = String::new();
    enum_str.push_str("pub enum PluginEnum {\n");
    for i in plugins.iter() {
        enum_str.push_str(&format!(
            "{}({}::{}),\n",
            i.plugin.name,
            i.cargo.package().name(),
            i.plugin.name
        ));
    }
    enum_str.push_str("}\n");
    enum_str
}

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
struct PluginCollection {
    content: Vec<PluginCrate>,
}

impl Deref for PluginCollection {
    type Target = Vec<PluginCrate>;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl DerefMut for PluginCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

#[derive(Serialize, Deserialize)]
struct PluginCrate {
    path: PathBuf,
    cargo: Manifest,
    plugin: PluginConfig,
}

#[derive(Serialize, Deserialize)]
struct PluginConfig {
    name: String,
    features: Vec<PluginFeature>,
}

#[derive(Serialize, Deserialize, PartialEq)]
enum PluginFeature {
    Gui,
}
