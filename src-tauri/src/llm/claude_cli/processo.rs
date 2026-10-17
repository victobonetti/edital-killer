//! Criação do processo `claude`, comum ao health check e ao runner.

use std::path::Path;
use std::process::Stdio;

use tokio::process::Command;

/// `claude` sem janela de console, com saídas capturadas e morto se o handle cair.
pub fn comando(claude: &Path) -> Command {
    let mut cmd = Command::new(claude);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
