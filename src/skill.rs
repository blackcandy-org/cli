use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

pub const CONTENT: &str = include_str!("../skills/blackcandy/SKILL.md");

pub fn install() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine the home directory")?;
    install_to(home.join(".agents/skills/blackcandy"))
}

fn install_to(directory: PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(&directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;

    let path = directory.join("SKILL.md");
    fs::write(&path, CONTENT).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_skill_has_expected_frontmatter() {
        assert!(CONTENT.starts_with("---\nname: blackcandy\n"));
        assert!(CONTENT.contains("description:"));
    }

    #[test]
    fn installs_embedded_skill() {
        let root = std::env::temp_dir().join(format!(
            "blackcandy-skill-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        let _ = fs::remove_dir_all(&root);

        let path = install_to(root.clone()).expect("skill should install");
        assert_eq!(fs::read_to_string(path).unwrap(), CONTENT);

        fs::remove_dir_all(root).unwrap();
    }
}
