use std::fs;
use tempfile::TempDir;

fn declared(root: &std::path::Path, folder: &str, id: &str) {
    let module = root.join("modules").join(folder);
    fs::create_dir_all(module.join(".ai-gate")).unwrap();
    fs::write(
        module.join(".project.json"),
        format!(r#"{{"id":"{id}","stack":"rust","profile":"test"}}"#),
    )
    .unwrap();
    fs::write(module.join(".ai-gate/gate.yaml"), "version: 1\n").unwrap();
}

#[test]
fn shared_catalog_is_sorted_and_exposes_only_relative_wire_paths() {
    let root = TempDir::new().unwrap();
    declared(root.path(), "zeta", "zeta");
    declared(root.path(), "alpha", "alpha");
    let modules = forge::module_catalog::discover_from(root.path()).unwrap();
    assert_eq!(
        modules.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    let json = serde_json::to_string(&modules).unwrap();
    assert!(json.contains("modules/alpha"));
    assert!(!json.contains(root.path().to_str().unwrap()));
}

#[test]
fn shared_catalog_rejects_duplicate_ids_and_missing_gate_declarations() {
    let root = TempDir::new().unwrap();
    declared(root.path(), "one", "same");
    declared(root.path(), "two", "same");
    assert!(matches!(
        forge::module_catalog::discover_from(root.path()),
        Err(forge::core::ForgeError::ModuleCatalogInvalid { .. })
    ));

    let root = TempDir::new().unwrap();
    let module = root.path().join("modules/no-gate");
    fs::create_dir_all(&module).unwrap();
    fs::write(
        module.join(".project.json"),
        r#"{"id":"no-gate","stack":"rust","profile":"test"}"#,
    )
    .unwrap();
    assert!(matches!(
        forge::module_catalog::discover_from(root.path()),
        Err(forge::core::ForgeError::ModuleCatalogInvalid { .. })
    ));
}

#[cfg(unix)]
#[test]
fn shared_catalog_rejects_module_symlinks_that_escape_modules() {
    use std::os::unix::fs::symlink;
    let root = TempDir::new().unwrap();
    fs::create_dir_all(root.path().join("modules")).unwrap();
    let outside = TempDir::new().unwrap();
    declared(outside.path(), "module", "module");
    symlink(
        outside.path().join("modules/module"),
        root.path().join("modules/escape"),
    )
    .unwrap();
    assert!(matches!(
        forge::module_catalog::discover_from(root.path()),
        Err(forge::core::ForgeError::ModuleCatalogInvalid { .. })
    ));
}
