use crate::{Error, InstallerKind, Result, Update};
use fs_err as fs;
use std::{path::PathBuf, process::Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxInstallCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl LinuxInstallCommand {
    pub fn for_kind(kind: InstallerKind, artifact: PathBuf) -> Result<Self> {
        let path = artifact.display().to_string();
        match kind {
            InstallerKind::AppImage => Ok(Self {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!("install -m 755 {path} {path}.new && mv {path}.new {path}"),
                ],
            }),
            InstallerKind::Deb => Ok(Self {
                program: "pkexec".into(),
                args: vec!["dpkg".into(), "-i".into(), path],
            }),
            InstallerKind::Rpm => Ok(Self {
                program: "pkexec".into(),
                args: vec!["rpm".into(), "-U".into(), path],
            }),
            _ => unreachable!("non-linux installer kind"),
        }
    }
}

impl Update {
    pub(crate) fn install_linux(&self, bytes: &[u8]) -> Result<()> {
        let staging_dir = tempfile::Builder::new()
            .prefix("release-hub-linux-installer-")
            .tempdir()?;
        let artifact_name = self
            .download_url
            .path_segments()
            .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
            .unwrap_or("release-hub-installer.bin");
        let artifact_path = staging_dir.path().join(artifact_name);

        fs::write(&artifact_path, bytes)?;

        let command = LinuxInstallCommand::for_kind(self.installer_kind.clone(), artifact_path)?;
        let status = Command::new(&command.program)
            .args(&command.args)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(Error::InstallerExecutionFailed(status.code().unwrap_or(-1)))
        }
    }
}
