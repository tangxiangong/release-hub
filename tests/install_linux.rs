use release_hub::{InstallerKind, LinuxInstallCommand};
use std::path::PathBuf;

#[test]
fn linux_deb_backend_builds_expected_install_command() {
    let command =
        LinuxInstallCommand::for_kind(InstallerKind::Deb, PathBuf::from("/tmp/release-hub.deb"))
            .unwrap();

    assert_eq!(command.program, "pkexec");
    assert_eq!(command.args, vec!["dpkg", "-i", "/tmp/release-hub.deb"]);
}
