//! Offline contract for the checksum-verified installer embedded in the
//! current Forge standard packs.

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::{Command, Output};

    const VERSION: &str = "0.1.0";
    const TARGET: &str = "x86_64-unknown-linux-gnu";
    const ARCHIVE: &str = "forge-0.1.0-x86_64-unknown-linux-gnu.tar.gz";
    const MEMBER: &str = "forge-0.1.0-x86_64-unknown-linux-gnu/forge";

    fn installer_source() -> &'static str {
        include_str!("../standards/baseline-service/1.4.0/install-forge.sh")
    }

    struct Fixture {
        _root: tempfile::TempDir,
        run_temp: PathBuf,
        assets: PathBuf,
        bin: PathBuf,
        github_path: PathBuf,
        installer: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let run_temp = root.path().join("runner-temp");
            let assets = root.path().join("assets");
            let bin = root.path().join("bin");
            fs::create_dir_all(&run_temp).unwrap();
            fs::create_dir_all(&assets).unwrap();
            fs::create_dir_all(&bin).unwrap();
            let github_path = root.path().join("github-path");
            fs::write(&github_path, "").unwrap();
            let installer = root.path().join("install-forge.sh");
            fs::write(&installer, installer_source()).unwrap();
            Self {
                _root: root,
                run_temp,
                assets,
                bin,
                github_path,
                installer,
            }
        }

        fn create_archive(&self, contents: &str) {
            self.create_archive_with_member(MEMBER, contents);
        }

        fn create_archive_with_member(&self, member: &str, contents: &str) {
            let root = self._root.path().join("archive-root");
            let binary = root.join(member);
            fs::create_dir_all(binary.parent().unwrap()).unwrap();
            fs::write(&binary, contents).unwrap();
            let mut permissions = fs::metadata(&binary).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&binary, permissions).unwrap();
            let archive_path = self.assets.join(ARCHIVE);
            let output = Command::new("tar")
                .current_dir(&root)
                .args(["-czf", archive_path.to_str().unwrap(), member])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let digest_output = Command::new("sha256sum")
                .arg(&archive_path)
                .output()
                .unwrap();
            assert!(digest_output.status.success());
            let digest_line = String::from_utf8(digest_output.stdout).unwrap();
            let digest = digest_line.split_whitespace().next().unwrap();
            fs::write(
                self.assets.join(format!("{ARCHIVE}.sha256")),
                format!("{digest}  {ARCHIVE}\n"),
            )
            .unwrap();
        }

        fn write_curl_stub(&self) {
            let curl = self.bin.join("curl");
            fs::write(
                &curl,
                r##"#!/bin/sh
set -eu
output=""
url=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output) output="$2"; shift 2 ;;
    *) url="$1"; shift ;;
  esac
done
name="${url##*/}"
[ -f "$FORGE_TEST_ASSETS/$name" ] || { echo "missing fixture $name" >&2; exit 22; }
cp "$FORGE_TEST_ASSETS/$name" "$output"
"##,
            )
            .unwrap();
            let mut permissions = fs::metadata(&curl).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(curl, permissions).unwrap();
        }

        fn run(&self, include_actions_path: bool) -> Output {
            self.write_curl_stub();
            let path = format!("{}:/usr/bin:/bin", self.bin.display());
            let mut cmd = Command::new("sh");
            cmd.arg(&self.installer)
                .env("PATH", path)
                .env("RUNNER_TEMP", &self.run_temp)
                .env("FORGE_TEST_ASSETS", &self.assets);
            if include_actions_path {
                cmd.env("GITHUB_PATH", &self.github_path);
            }
            cmd.output().unwrap()
        }

        fn installed_binary(&self) -> PathBuf {
            self.run_temp
                .join(format!("forge-{VERSION}"))
                .join("bin/forge")
        }
    }

    fn assert_no_install_or_path(fixture: &Fixture) {
        assert!(!fixture.installed_binary().exists());
        assert_eq!(fs::read_to_string(&fixture.github_path).unwrap(), "");
    }

    #[test]
    fn downloads_verifies_and_installs_before_exposing_binary_on_github_path() {
        assert_eq!(TARGET, "x86_64-unknown-linux-gnu");
        let fixture = Fixture::new();
        fixture.create_archive("#!/bin/sh\nprintf 'forge 0.1.0\\n'\n");
        let output = fixture.run(true);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let binary = fixture.installed_binary();
        assert!(binary.is_file());
        assert_eq!(
            fs::read_to_string(&fixture.github_path).unwrap(),
            format!("{}\n", binary.parent().unwrap().display())
        );
        let version = Command::new(binary).output().unwrap();
        assert!(version.status.success());
        assert_eq!(String::from_utf8(version.stdout).unwrap(), "forge 0.1.0\n");
    }

    #[test]
    fn rejects_bad_checksum_sidecars_without_install_or_path_mutation() {
        for sidecar in [
            format!("{}  another-archive.tar.gz\n", "0".repeat(64)),
            format!("{}  {ARCHIVE}\nextra\n", "0".repeat(64)),
            format!("{}  {ARCHIVE}\n", "0".repeat(63)),
        ] {
            let fixture = Fixture::new();
            fixture.create_archive("#!/bin/sh\nexit 0\n");
            fs::write(fixture.assets.join(format!("{ARCHIVE}.sha256")), sidecar).unwrap();
            let output = fixture.run(true);
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("checksum"));
            assert_no_install_or_path(&fixture);
        }
    }

    #[test]
    fn refuses_missing_assets_actions_environment_and_unexpected_archive_member() {
        let missing = Fixture::new();
        let output = missing.run(true);
        assert!(!output.status.success());
        assert_no_install_or_path(&missing);

        let missing_path = Fixture::new();
        missing_path.create_archive("#!/bin/sh\nexit 0\n");
        let output = missing_path.run(false);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("GITHUB_PATH"));
        assert_no_install_or_path(&missing_path);

        let unexpected = Fixture::new();
        unexpected.create_archive_with_member("wrong/path/forge", "#!/bin/sh\nexit 0\n");
        let output = unexpected.run(true);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("must contain exactly one"));
        assert_no_install_or_path(&unexpected);
    }
}
