use std::io;
use std::path::Path;

pub fn write_pid(path: &Path) -> io::Result<()> {
    let pid = std::process::id().to_string();
    std::fs::write(path, pid)
}

pub fn read_pid(path: &Path) -> io::Result<u32> {
    let content = std::fs::read_to_string(path)?;
    content
        .trim()
        .parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn remove_pid(path: &Path) -> io::Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub fn is_running(path: &Path) -> bool {
    let Ok(pid) = read_pid(path) else {
        return false;
    };
    unsafe {
        let ret = libc::kill(pid as libc::pid_t, 0);
        ret == 0
    }
}
