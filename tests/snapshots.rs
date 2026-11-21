//! Snapshot tests of `--plain` output for every fixture.
//!
//! Review changes with `cargo insta review` (install: `cargo install cargo-insta`).

use std::path::Path;

fn render(name: &str, width: usize, color: bool) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    let src = std::fs::read_to_string(&path).unwrap();
    // Normalize line endings so Windows checkouts produce the same output.
    mdpeek::render_plain(&src.replace("\r\n", "\n"), width, color)
}

macro_rules! snap {
    ($test:ident, $file:literal, $width:literal) => {
        #[test]
        fn $test() {
            insta::assert_snapshot!(render($file, $width, false));
        }
    };
}

snap!(basics_60, "basics.md", 60);
snap!(lists_60, "lists.md", 60);
snap!(lists_30, "lists.md", 30);
snap!(code_60, "code.md", 60);
snap!(tables_60, "tables.md", 60);
snap!(tables_30, "tables.md", 30);
snap!(images_60, "images.md", 60);

#[test]
fn code_color() {
    insta::assert_snapshot!(render("code.md", 60, true));
}
