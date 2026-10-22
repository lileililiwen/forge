//! Profile build-command truth contract (`profile-build-command-truth`).
//!
//! A profile's declared build command must build. The catalogue's build
//! commands are pinned here, so a build command cannot change in the
//! catalogue without this check failing and naming the profile. A declared
//! build command whose verb is a named analysis verb is refused with
//! `invalid-profile` at both declaration doors: an external descriptor
//! document and the built-in catalogue.

use forge::profile::{
    build_command_verb, descriptor_from_yaml, inspect_profile, list_profiles, ANALYSIS_BUILD_VERBS,
};

/// `(profile, declared build command)` for every supported profile.
const MATRIX: [(&str, &str); 6] = [
    ("aspnet-web", "dotnet build"),
    ("flutter-app", "flutter build bundle"),
    ("nextjs-web", "npm run build"),
    ("python-service", "python3 -m compileall -q app tests"),
    ("react-web", "npm run build"),
    ("rust-web", "cargo build"),
];

/// An external descriptor document with the given build command.
fn descriptor_yaml(build_command: &str) -> Vec<u8> {
    format!(
        "id: custom-web\nversion: 0.1.0\nadapter: adapter-custom\nlanguage: rust\n\
         toolchain: cargo\nbuild_command: {build_command}\ntest_command: cargo test\n"
    )
    .into_bytes()
}

#[test]
fn supported_build_commands_match_the_pinned_table() {
    for (profile, command) in MATRIX {
        let descriptor = inspect_profile(profile)
            .unwrap_or_else(|err| panic!("{profile} must be inspectable: {err}"));
        assert_eq!(
            descriptor.build_command, command,
            "{profile} must declare the pinned build command"
        );
    }
    // The pinned table covers the whole supported catalogue: a new
    // supported profile must be added to it.
    let supported = list_profiles();
    assert_eq!(
        supported.len(),
        MATRIX.len(),
        "the pinned table must cover every supported profile"
    );
    for descriptor in &supported {
        assert!(
            MATRIX.iter().any(|(id, _)| *id == descriptor.id),
            "{} must be pinned in the table",
            descriptor.id
        );
    }
}

#[test]
fn no_supported_build_command_carries_an_analysis_verb() {
    for descriptor in list_profiles() {
        let verb = build_command_verb(&descriptor.build_command)
            .unwrap_or_else(|| panic!("{} declares no build command", descriptor.id));
        assert!(
            !ANALYSIS_BUILD_VERBS.contains(&verb),
            "{} declares the analysis verb `{}` as its build command",
            descriptor.id,
            verb
        );
    }
}

#[test]
fn the_verb_is_read_past_the_flags() {
    assert_eq!(
        build_command_verb("cargo clippy --all-targets"),
        Some("clippy")
    );
    assert_eq!(
        build_command_verb("npm run build -- --mode=production"),
        Some("build")
    );
    assert_eq!(build_command_verb("python3 -m build"), Some("build"));
    assert_eq!(
        build_command_verb("flutter analyze --fatal-infos"),
        Some("analyze")
    );
}

#[test]
fn an_external_descriptor_with_an_analysis_build_command_is_refused() {
    let err = descriptor_from_yaml(&descriptor_yaml("cargo clippy"))
        .expect_err("an analysis build command must be refused");
    assert_eq!(err.code(), "invalid-profile");
    let text = err.to_string();
    assert!(text.contains("custom-web"), "{text}");
    assert!(text.contains("cargo clippy"), "{text}");
    assert!(text.contains("clippy"), "{text}");
    assert!(text.contains("no files were changed"), "{text}");
}

#[test]
fn an_external_descriptor_with_a_build_command_is_accepted() {
    let parsed = descriptor_from_yaml(&descriptor_yaml("cargo build"))
        .expect("a build command must be accepted");
    assert_eq!(parsed.id, "custom-web");
    assert_eq!(parsed.build_command, "cargo build");
}

#[test]
fn the_builtin_flutter_catalogue_declares_a_real_build() {
    let flutter = inspect_profile("flutter-app").expect("flutter-app must be inspectable");
    assert_eq!(flutter.build_command, "flutter build bundle");
}
