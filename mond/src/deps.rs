use std::{
    path::{Path, PathBuf},
    process::Command,
};

use eyre::Context;
use walkdir::WalkDir;

use crate::{
    SOURCE_DIR, TARGET_DIR, manifest,
    ui::{info, success},
    utils::find_mond_files,
};

#[derive(Clone, Debug)]
pub(crate) struct HelperErlFile {
    pub(crate) file_name: String,
    pub(crate) module_name: String,
    pub(crate) contents: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StdDependency {
    pub(crate) modules: Vec<(String, String, String)>,
    pub(crate) helper_erls: Vec<HelperErlFile>,
}

pub(crate) fn load_std_dependency(
    project_dir: &Path,
    manifest: &manifest::MondManifest,
) -> eyre::Result<StdDependency> {
    let Some(dep_spec) = manifest.dependencies.get("std") else {
        return Ok(StdDependency::default());
    };

    let checkout_dir = checkout_dependency(project_dir, "std", dep_spec)?;
    load_std_from_checkout(&checkout_dir)
}

pub(crate) fn update_dependencies(project_dir: &Path) -> eyre::Result<Vec<String>> {
    let manifest = manifest::read_manifest(project_dir.to_path_buf())?;
    let mut updated = Vec::new();

    let mut dep_names: Vec<String> = manifest.dependencies.keys().cloned().collect();
    dep_names.sort();

    for dep_name in dep_names {
        let dep_spec = &manifest.dependencies[&dep_name];
        checkout_dependency_with_policy(project_dir, &dep_name, dep_spec, true)?;
        updated.push(dep_name);
    }

    Ok(updated)
}

fn checkout_dependency(
    project_dir: &Path,
    dep_name: &str,
    dep_spec: &manifest::DependencySpec,
) -> eyre::Result<PathBuf> {
    checkout_dependency_with_policy(project_dir, dep_name, dep_spec, false)
}

fn checkout_dependency_with_policy(
    project_dir: &Path,
    dep_name: &str,
    dep_spec: &manifest::DependencySpec,
    refresh: bool,
) -> eyre::Result<PathBuf> {
    let deps_dir = project_dir.join(TARGET_DIR).join("deps");
    std::fs::create_dir_all(&deps_dir).with_context(|| {
        format!(
            "could not create dependency cache at {}",
            deps_dir.display()
        )
    })?;

    let checkout_dir = deps_dir.join(dep_name);
    let git_dir = checkout_dir.join(".git");
    let git_dir_exists = git_dir.exists();

    if git_dir_exists {
        if refresh {
            info(&format!("Fetching dependency: {dep_name}"));
            run_git(
                Some(&checkout_dir),
                &["fetch", "--quiet", "--tags", "--prune", "origin"],
                "failed to fetch dependency",
            )?;
            success(&format!("Fetched dependency: {dep_name}"));
        }
    } else if checkout_dir.exists() {
        return Err(eyre::eyre!(
            "dependency cache path {} exists but is not a git repository; remove it and retry",
            checkout_dir.display()
        ));
    } else {
        info(&format!("Cloning dependency: {dep_name}"));
        run_git(
            None,
            &[
                "clone",
                "--quiet",
                "--",
                dep_spec.git.as_str(),
                checkout_dir
                    .to_str()
                    .ok_or_else(|| eyre::eyre!("invalid checkout path"))?,
            ],
            "failed to clone dependency",
        )?;

        success(&format!("Cloned dependency: {dep_name}"));
    }

    // if the git dir already existed and we didn't refresh we're already on the right tag
    if git_dir_exists && !refresh {
        return Ok(checkout_dir);
    }

    match &dep_spec.reference {
        manifest::GitReference::Tag(tag) => {
            info(&format!(
                "Checking out dependency: {dep_name} using tag: {tag}"
            ));
            run_git(
                Some(&checkout_dir),
                &["checkout", "--quiet", &format!("refs/tags/{tag}")],
                "failed to checkout dependency tag",
            )?;
            success(&format!(
                "Checked out dependency: {dep_name} using tag: {tag}"
            ));
        }
        manifest::GitReference::Branch(branch) => {
            info(&format!(
                "Checking our dependency: {dep_name} using branch: {branch}"
            ));

            run_git(
                Some(&checkout_dir),
                &[
                    "checkout",
                    "--quiet",
                    "-B",
                    branch,
                    &format!("origin/{branch}"),
                ],
                "failed to checkout dependency branch",
            )?;

            success(&format!(
                "Checked out dependency: {dep_name} using brach: {branch}"
            ));
        }
        manifest::GitReference::Rev(rev) => {
            info(&format!(
                "Checking out dependency: {dep_name} using rev: {rev}"
            ));

            run_git(
                Some(&checkout_dir),
                &["checkout", "--quiet", rev],
                "failed to checkout dependency revision",
            )?;
            success(&format!(
                "Checked out dependency: {dep_name} using rev: {rev}"
            ));
        }
    }

    Ok(checkout_dir)
}

fn run_git(cwd: Option<&Path>, args: &[&str], context: &str) -> eyre::Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "alias.checkout=", "-c", "alias.switch="]);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .with_context(|| format!("{context}: could not run git"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(eyre::eyre!(
        "{context}: {}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    ))
}

fn load_std_from_checkout(checkout_dir: &Path) -> eyre::Result<StdDependency> {
    let dep_manifest = manifest::read_manifest(checkout_dir.to_path_buf())
        .with_context(|| format!("could not read std manifest at {}", checkout_dir.display()))?;
    if dep_manifest.package.name != "std" {
        return Err(eyre::eyre!(
            "dependency `std` points to package `{}`; expected `std`",
            dep_manifest.package.name
        ));
    }

    let src_dir = checkout_dir.join(SOURCE_DIR);
    if !src_dir.exists() {
        return Err(eyre::eyre!(
            "std dependency is missing `{}` at {}",
            SOURCE_DIR,
            src_dir.display()
        ));
    }

    let mut std_sources: Vec<(String, String)> = Vec::new();
    let mut lib_source: Option<String> = None;
    for mond_path in find_mond_files(&src_dir) {
        let module_name = mond_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let source = std::fs::read_to_string(&mond_path)
            .with_context(|| format!("could not read {}", mond_path.display()))?;
        if module_name == "lib" {
            lib_source = Some(source);
        } else {
            std_sources.push((module_name, source));
        }
    }
    if let Some(lib_src) = lib_source {
        std_sources.push(("std".to_string(), lib_src));
    }

    let modules = mondc::std_modules_from_sources(&std_sources).map_err(|err| eyre::eyre!(err))?;

    let mut helper_erls: Vec<HelperErlFile> = WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("erl"))
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?.to_string();
            let module_name = path.file_stem()?.to_str()?.to_string();
            let contents = std::fs::read(path).ok()?;
            Some(HelperErlFile {
                file_name,
                module_name,
                contents,
            })
        })
        .collect();
    helper_erls.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    Ok(StdDependency {
        modules,
        helper_erls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("mond-deps-test-{}-{nanos}", std::process::id()))
    }

    fn run_ok(cmd: &mut Command) {
        let output = cmd.output().expect("run command");
        assert!(
            output.status.success(),
            "command failed: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn load_std_dependency_returns_empty_without_std_dep() {
        let manifest = manifest::MondManifest {
            package: manifest::Package {
                name: "app".to_string(),
                version: Version::new(0, 1, 0),
                mond_version: Version::new(0, 1, 0),
            },
            dependencies: std::collections::HashMap::new(),
        };
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).expect("create temp root");
        let loaded = load_std_dependency(&root, &manifest).expect("load deps");
        assert!(loaded.modules.is_empty());
        assert!(loaded.helper_erls.is_empty());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn load_std_dependency_loads_from_git_tag() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).expect("create root");
        let std_repo = root.join("std-src");
        let std_src_dir = std_repo.join("src");
        std::fs::create_dir_all(&std_src_dir).expect("create std src");
        std::fs::write(
            std_repo.join("mond.toml"),
            r#"[package]
name = "std"
version = "0.0.1"
mond_version = "0.1.0"

[dependencies]
"#,
        )
        .expect("write std manifest");
        std::fs::write(std_src_dir.join("lib.mond"), "(pub let hello {} \"hello\")")
            .expect("write lib.mond");
        std::fs::write(std_src_dir.join("io.mond"), "(pub let println {x} x)")
            .expect("write io.mond");
        std::fs::write(
            std_src_dir.join("mond_std_helpers.erl"),
            "-module(mond_std_helpers).\n",
        )
        .expect("write helper");

        run_ok(Command::new("git").arg("init").current_dir(&std_repo));
        run_ok(
            Command::new("git")
                .args(["add", "."])
                .current_dir(&std_repo),
        );
        run_ok(
            Command::new("git")
                .args([
                    "-c",
                    "user.email=test@example.com",
                    "-c",
                    "user.name=test",
                    "commit",
                    "-m",
                    "initial",
                ])
                .current_dir(&std_repo),
        );
        run_ok(
            Command::new("git")
                .args(["tag", "0.0.1"])
                .current_dir(&std_repo),
        );

        let project_dir = root.join("app");
        std::fs::create_dir_all(&project_dir).expect("create project");
        let manifest = manifest::MondManifest {
            package: manifest::Package {
                name: "app".to_string(),
                version: Version::new(0, 1, 0),
                mond_version: Version::new(0, 1, 0),
            },
            dependencies: std::collections::HashMap::from([(
                "std".to_string(),
                manifest::DependencySpec {
                    git: format!("file://{}", std_repo.display()),
                    reference: manifest::GitReference::Tag("0.0.1".to_string()),
                },
            )]),
        };

        let loaded = load_std_dependency(&project_dir, &manifest).expect("load std dep");
        let names: std::collections::HashSet<String> = loaded
            .modules
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect();
        assert!(names.contains("std"));
        assert!(names.contains("io"));
        assert!(
            loaded
                .helper_erls
                .iter()
                .any(|h| h.file_name == "mond_std_helpers.erl")
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn load_std_dependency_uses_cached_checkout_without_fetching() {
        let root = unique_temp_root();
        std::fs::create_dir_all(&root).expect("create root");
        let std_repo = root.join("std-src");
        let std_src_dir = std_repo.join("src");
        std::fs::create_dir_all(&std_src_dir).expect("create std src");
        std::fs::write(
            std_repo.join("mond.toml"),
            r#"[package]
name = "std"
version = "0.0.1"
mond_version = "0.1.0"

[dependencies]
"#,
        )
        .expect("write std manifest");
        std::fs::write(std_src_dir.join("lib.mond"), "(pub let hello {} \"hello\")")
            .expect("write lib.mond");

        run_ok(Command::new("git").arg("init").current_dir(&std_repo));
        run_ok(
            Command::new("git")
                .args(["add", "."])
                .current_dir(&std_repo),
        );
        run_ok(
            Command::new("git")
                .args([
                    "-c",
                    "user.email=test@example.com",
                    "-c",
                    "user.name=test",
                    "commit",
                    "-m",
                    "initial",
                ])
                .current_dir(&std_repo),
        );
        run_ok(
            Command::new("git")
                .args(["tag", "0.0.1"])
                .current_dir(&std_repo),
        );

        let project_dir = root.join("app");
        std::fs::create_dir_all(&project_dir).expect("create project");
        let manifest = manifest::MondManifest {
            package: manifest::Package {
                name: "app".to_string(),
                version: Version::new(0, 1, 0),
                mond_version: Version::new(0, 1, 0),
            },
            dependencies: std::collections::HashMap::from([(
                "std".to_string(),
                manifest::DependencySpec {
                    git: format!("file://{}", std_repo.display()),
                    reference: manifest::GitReference::Tag("0.0.1".to_string()),
                },
            )]),
        };

        let initial = load_std_dependency(&project_dir, &manifest).expect("initial load");
        assert!(
            initial.modules.iter().any(|(name, _, _)| name == "std"),
            "expected initial clone to load std"
        );

        std::fs::remove_dir_all(&std_repo).expect("remove remote repo");

        let cached = load_std_dependency(&project_dir, &manifest).expect("cached load");
        assert!(
            cached.modules.iter().any(|(name, _, _)| name == "std"),
            "expected cached checkout to be used without fetching"
        );

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
