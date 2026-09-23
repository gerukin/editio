//! Compile with rustc and --extern yaml_rust, then pass Linguist languages.yml and output path.
use std::{collections::BTreeMap, env, fs};
fn main() {
    let args: Vec<_> = env::args().collect();
    let yaml = yaml_rust::YamlLoader::load_from_str(&fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut aliases = BTreeMap::new();
    for (name, data) in yaml[0].as_hash().unwrap() {
        let name = name.as_str().unwrap();
        let scope = data["tm_scope"].as_str().unwrap_or("none");
        let extensions = data["extensions"].as_vec().map(|v| v.iter().filter_map(|x| x.as_str()).map(|s| s.trim_start_matches('.')).collect::<Vec<_>>().join(",")).unwrap_or_default();
        let mut names = vec![name.to_lowercase(), name.to_lowercase().replace(' ', "-")];
        if let Some(values) = data["aliases"].as_vec() { names.extend(values.iter().filter_map(|x| x.as_str()).map(str::to_lowercase)); }
        for alias in names { aliases.insert(alias,format!("{name}\t{scope}\t{extensions}")); }
    }
    let mut out = String::from("# alias\tGitHub language\tTextMate scope\textensions\n");
    for (alias,data) in aliases { out.push_str(&format!("{alias}\t{data}\n")); }
    fs::write(&args[2],out).unwrap();
}
