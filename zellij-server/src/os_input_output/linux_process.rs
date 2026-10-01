use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub(super) fn get_cwd(proc_root: &Path, pid: u32) -> Option<PathBuf> {
    match fs::read_link(proc_root.join(pid.to_string()).join("cwd")) {
        Ok(cwd) => Some(cwd),
        Err(error) => {
            log::debug!("Failed to read cwd for process {pid}: {error}");
            None
        },
    }
}

pub(super) fn get_cwds(
    proc_root: &Path,
    pids: Vec<u32>,
) -> (HashMap<u32, PathBuf>, HashMap<u32, Vec<String>>) {
    let mut cwds = HashMap::new();
    let mut cmds = HashMap::new();
    for pid in pids {
        if let Some(cwd) = get_cwd(proc_root, pid) {
            cwds.insert(pid, cwd);
        }
        if let Some(cmd) = get_cmd(proc_root, pid) {
            cmds.insert(pid, cmd);
        }
    }
    (cwds, cmds)
}

pub(super) fn get_cmd(proc_root: &Path, pid: u32) -> Option<Vec<String>> {
    match fs::read(proc_root.join(pid.to_string()).join("cmdline")) {
        Ok(cmdline) => {
            let cmd: Vec<String> = cmdline
                .split(|byte| *byte == 0)
                .map(|argument| argument.trim_ascii())
                .filter(|argument| !argument.is_empty())
                .map(|argument| String::from_utf8_lossy(argument).into_owned())
                .collect();
            match cmd.is_empty() {
                true => None,
                false => Some(cmd),
            }
        },
        Err(error) => {
            log::debug!("Failed to read cmdline for process {pid}: {error}");
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{ffi::OsStringExt, fs::symlink};

    #[test]
    fn reads_only_requested_processes() {
        let proc_root = tempfile::tempdir().unwrap();
        let pid_dir = proc_root.path().join("42");
        fs::create_dir(&pid_dir).unwrap();
        let cwd = PathBuf::from("/directory with spaces");
        symlink(&cwd, pid_dir.join("cwd")).unwrap();
        fs::write(
            pid_dir.join("cmdline"),
            b"/bin/program\0argument with spaces\0--flag\0",
        )
        .unwrap();
        fs::write(proc_root.path().join("unrelated"), b"not a process").unwrap();

        let (cwds, cmds) = get_cwds(proc_root.path(), vec![42, 43]);

        assert_eq!(cwds, HashMap::from([(42, cwd)]));
        assert_eq!(
            cmds,
            HashMap::from([(
                42,
                vec![
                    "/bin/program".to_owned(),
                    "argument with spaces".to_owned(),
                    "--flag".to_owned()
                ]
            )])
        );
    }

    #[test]
    fn preserves_non_utf8_cwd_and_lossy_command_conversion() {
        let proc_root = tempfile::tempdir().unwrap();
        let pid_dir = proc_root.path().join("42");
        fs::create_dir(&pid_dir).unwrap();
        let cwd = PathBuf::from(std::ffi::OsString::from_vec(b"/directory-\xff".to_vec()));
        symlink(&cwd, pid_dir.join("cwd")).unwrap();
        fs::write(
            pid_dir.join("cmdline"),
            b"/bin/program\0\0  argument  \0\xff\0",
        )
        .unwrap();

        let (cwds, cmds) = get_cwds(proc_root.path(), vec![42]);

        assert_eq!(cwds.get(&42), Some(&cwd));
        assert_eq!(
            cmds.get(&42),
            Some(&vec![
                "/bin/program".to_owned(),
                "argument".to_owned(),
                String::from_utf8_lossy(b"\xff").into_owned()
            ])
        );
    }

    #[test]
    fn missing_cwd_does_not_discard_command() {
        let proc_root = tempfile::tempdir().unwrap();
        let pid_dir = proc_root.path().join("42");
        fs::create_dir(&pid_dir).unwrap();
        fs::write(pid_dir.join("cmdline"), b"/bin/program\0").unwrap();

        let (cwds, cmds) = get_cwds(proc_root.path(), vec![42]);

        assert!(cwds.is_empty());
        assert_eq!(cmds.get(&42), Some(&vec!["/bin/program".to_owned()]));
        assert_eq!(get_cwd(proc_root.path(), 42), None);
    }

    #[test]
    fn missing_or_empty_command_does_not_discard_cwd() {
        let proc_root = tempfile::tempdir().unwrap();
        for pid in [42, 43] {
            let pid_dir = proc_root.path().join(pid.to_string());
            fs::create_dir(&pid_dir).unwrap();
            symlink("/directory", pid_dir.join("cwd")).unwrap();
        }
        fs::write(proc_root.path().join("43/cmdline"), b"").unwrap();

        let (cwds, cmds) = get_cwds(proc_root.path(), vec![42, 43]);

        assert_eq!(cwds.len(), 2);
        assert!(cmds.is_empty());
    }

    #[test]
    fn empty_batch_does_not_access_proc_root() {
        let (cwds, cmds) = get_cwds(Path::new("/nonexistent-proc-root"), vec![]);
        assert!(cwds.is_empty());
        assert!(cmds.is_empty());
    }
}
