//! Optional build-time embedding for `storytold/craft-fonts` (`CRAFT_FONTS_DIR`) and
//! `storytold/craft-i18n` (`CRAFT_I18N_DIR`), following `craft-fonts-build/README.md` and
//! `craft-i18n/README.md`. Neither data repository is ever listed in `Cargo.toml`; when the
//! environment variables are unset, Magic Studio builds with empty embedded tables and English UI.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_REQUIRED");
    println!("cargo::rerun-if-env-changed=CRAFT_I18N_DIR");

    let wasm = craft_fonts_build::target_is_wasm();
    let embedded = craft_fonts_build::Embed::new().subset_to(catalogs("magic")).filter(move |face| !wasm || face.has_script("Jpan")).write("craft_fonts.rs");
    if let Err(e) = embedded {
        println!("cargo::error={e}");
    }

    if let Err(e) = write_i18n_table("magic") {
        println!("cargo::error={e}");
    }
}

fn write_out_file(dest: &Path, content: &str) -> Result<(), String> {
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes()).map_err(|e| e.to_string())
}

fn catalogs(app: &str) -> Vec<PathBuf> {
    let Some(dir) = std::env::var_os("CRAFT_I18N_DIR").filter(|dir| !dir.is_empty()) else {
        return Vec::new();
    };
    let locales = PathBuf::from(dir).join("locales");
    println!("cargo::rerun-if-changed={}", locales.display());
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&locales)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|locale| ["common.tsv".to_owned(), format!("{app}.tsv")].map(|file| locale.path().join(file)))
        .filter(|path| path.is_file())
        .collect();
    paths.sort();
    paths
}

fn write_i18n_table(app: &str) -> Result<(), String> {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let dest = out_dir.join("craft_i18n.rs");
    let Some(dir) = std::env::var_os("CRAFT_I18N_DIR").filter(|d| !d.is_empty()) else {
        return write_out_file(&dest, "pub static CRAFT_I18N: &craft_i18n::Embedded = &[];\n");
    };
    let locales_dir = PathBuf::from(dir).join("locales");
    let mut entries: Vec<(String, PathBuf, Option<PathBuf>)> = Vec::new();
    for entry in std::fs::read_dir(&locales_dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let Some(code) = p.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        let common = p.join("common.tsv");
        if !common.is_file() {
            continue;
        }
        let app_tsv = p.join(format!("{app}.tsv"));
        entries.push((code, common, app_tsv.is_file().then_some(app_tsv)));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut code = String::from("pub static CRAFT_I18N: &craft_i18n::Embedded = &[\n");
    for (loc, common, app_opt) in entries {
        let common_disp = common.display().to_string().replace('\\', "/");
        match app_opt {
            Some(app_file) => {
                let app_disp = app_file.display().to_string().replace('\\', "/");
                let _ = writeln!(code, "    ({loc:?}, include_str!({common_disp:?}), include_str!({app_disp:?})),");
            }
            None => {
                let _ = writeln!(code, "    ({loc:?}, include_str!({common_disp:?}), \"\"),");
            }
        }
    }
    code.push_str("];\n");
    write_out_file(&dest, &code)
}
