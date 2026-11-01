use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn quiet(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

pub fn run(program: &Path, args: impl IntoIterator<Item = String>) -> io::Result<Option<i32>> {
    let status = quiet(Command::new(program).args(args)).status()?;
    Ok(status.code())
}

pub fn launch(program: &Path, args: &[String]) -> io::Result<()> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(folder) = program.parent() {
        command.current_dir(folder);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

pub fn reveal(folder: &Path) -> io::Result<()> {
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    Command::new(opener).arg(folder).spawn().map(drop)
}

#[must_use]
pub fn is_running(program: &Path) -> bool {
    let Some(name) = program.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    #[cfg(windows)]
    {
        let filter = format!("IMAGENAME eq {name}");
        let mut command = Command::new("tasklist");
        command.args(["/FO", "CSV", "/NH", "/FI", filter.as_str()]);
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command.output().is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .to_ascii_lowercase()
                .contains(&format!("\"{}\"", name.to_ascii_lowercase()))
        })
    }
    #[cfg(not(windows))]
    {
        std::fs::read_dir("/proc").is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                std::fs::read_link(entry.path().join("exe"))
                    .is_ok_and(|exe| exe.file_name().is_some_and(|found| found == name))
            })
        })
    }
}

pub fn make_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
