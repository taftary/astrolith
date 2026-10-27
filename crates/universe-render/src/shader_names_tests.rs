//! Shader module names (#1029): naga_oil refuses, in debug builds only, a
//! module that names something naga's writeback would rename (a name
//! ending in a digit, like `len_c2`). The shader then fails to load and
//! the ground is not drawn, while release builds skip the check. This test
//! reads every module the game imports and names any such declaration.

/// The game's WGSL files, by name and text.
const SHADERS: [(&str, &str); 3] = [
    ("patch_grid.wgsl", include_str!("patch_grid.wgsl")),
    (
        "patch_grid_prepass.wgsl",
        include_str!("patch_grid_prepass.wgsl"),
    ),
    (
        "patch_grid_common.wgsl",
        include_str!("patch_grid_common.wgsl"),
    ),
];

/// Names `text` declares: after `fn`, `let`, `var`, `const`, `override`,
/// `struct` or `alias`, and before a single `:` (fields and parameters).
fn declared(text: &str) -> Vec<String> {
    let code: String = text
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let chars: Vec<char> = code.chars().collect();
    let mut names = Vec::new();
    let mut previous = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let single_colon = chars.get(j) == Some(&':') && chars.get(j + 1) != Some(&':');
            let after_keyword = matches!(
                previous.as_str(),
                "fn" | "let" | "var" | "const" | "override" | "struct" | "alias"
            );
            if single_colon || after_keyword {
                names.push(word.clone());
            }
            previous = word;
        } else {
            if !c.is_whitespace() {
                previous.clear();
            }
            i += 1;
        }
    }
    names
}

#[test]
fn imported_shader_modules_use_no_names_naga_would_rename() {
    let mut bad = Vec::new();
    for (file, text) in SHADERS {
        if !text.contains("#define_import_path") {
            continue;
        }
        for name in declared(text) {
            if name.ends_with(|c: char| c.is_ascii_digit()) {
                bad.push(format!("{file}: {name}"));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "names ending in a digit break the module in debug builds: {bad:?}"
    );
}

#[test]
fn the_name_check_finds_fields_and_locals() {
    let names = declared(
        "struct Tile {\n    len_c2: f32,\n    page: i32,\n};\nfn f(a: vec2<f32>) -> f32 {\n    let dlen2 = 1.0; // x9: no\n    var<private> w3: f32;\n    return a.x;\n}\n",
    );
    for want in ["Tile", "len_c2", "page", "f", "a", "dlen2", "w3"] {
        assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
    }
    assert!(!names.iter().any(|n| n == "x9" || n == "vec2" || n == "f32"));
}
