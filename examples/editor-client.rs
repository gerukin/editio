//! Interactive synthetic caller; never changes editor defaults.
use std::{io, process::Command};
fn main() -> io::Result<()> {
    let editor = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "editio".into());
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("draft message.txt");
    std::fs::write(
        &path,
        "Synthetic client draft. Replace this text if desired.\n",
    )?;
    println!("Temporary draft: {}", path.display());
    println!("Test normal quit, save + quit, or Ctrl+K → Abort editing → a.");
    let status = Command::new(editor).arg("--").arg(&path).status()?;
    println!("Editor exit status: {status}");
    if status.success() {
        println!("CLIENT ACCEPTED:\n{}", std::fs::read_to_string(path)?);
    } else {
        println!(
            "CLIENT CANCELLED. Nothing submitted. Last saved draft:\n{}",
            std::fs::read_to_string(path)?
        );
    }
    Ok(())
}
