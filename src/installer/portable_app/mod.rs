mod desktop;
mod extract;
mod paths;
mod service;
mod symlink;
mod uninstall;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PortableAppConfig {
    /// Application name (used for directory name, bin symlink, etc.)
    pub app_name: String,
    /// Relative path of the executable inside the package (e.g. "bin/code")
    pub bin_rel_path: String,
    /// Relative path of the icon inside the package (e.g. "share/icons/code.png")
    pub icon_rel_path: Option<String>,
    /// Path to an external icon file (copied into app dir)
    pub cicon_path: Option<PathBuf>,
    /// Generate a systemd user service
    pub service: bool,
    /// Enable autostart via .desktop symlink
    pub autostart: bool,
    /// Whether to skip confirmation when app_dir already exists
    pub yes: bool,
    /// Custom app directory root (default: ~/.local/app)
    pub app_root: Option<PathBuf>,
}

#[derive(Debug)]
pub struct InstallResult {
    pub app_dir: PathBuf,
    pub bin_link: PathBuf,
    pub desktop_link: PathBuf,
    pub service_link: Option<PathBuf>,
    pub autostart_link: Option<PathBuf>,
    pub uninstall_path: PathBuf,
}

impl InstallResult {
    pub fn summary(&self, app_name: &str) -> String {
        let mut lines = vec![
            format!("  App directory: {}", self.app_dir.display()),
            format!("  Uninstall:     {}", self.uninstall_path.display()),
            format!("  Binary:        {}", self.bin_link.display()),
            format!("  Desktop:       {}", self.desktop_link.display()),
        ];
        if let Some(ref s) = self.service_link {
            lines.push(format!("  Service:       {}", s.display()));
            lines.push(format!(
                "  Run: systemctl --user start {}.service",
                app_name
            ));
        }
        if let Some(ref a) = self.autostart_link {
            lines.push(format!("  Autostart:     {}", a.display()));
        }
        lines.join("\n")
    }
}

pub struct PortableAppInstaller;

impl PortableAppInstaller {
    /// Run the full portable app installation pipeline.
    ///
    /// `pkg_path` - path to the tar.gz package
    pub fn run(pkg_path: &Path, config: PortableAppConfig) -> Result<InstallResult, String> {
        let home = paths::home_dir().map_err(|e| format!("Failed to determine home: {}", e))?;
        let app_root = config
            .app_root
            .clone()
            .unwrap_or_else(|| home.join(".local").join("app"));
        let bin_dir = home.join(".local").join("bin");
        let desktop_dir = home.join(".local").join("share").join("applications");
        let systemd_dir = home.join(".config").join("systemd").join("user");
        let autostart_dir = home.join(".config").join("autostart");

        let app_name = &config.app_name;
        let app_dir = app_root.join(app_name);

        // ── Handle existing app directory ──
        if app_dir.exists() {
            if !config.yes {
                return Err(format!(
                    "App directory already exists: {}\nUse --yes to overwrite automatically, or remove it first.",
                    app_dir.display()
                ));
            }
            std::fs::remove_dir_all(&app_dir)
                .map_err(|e| format!("Failed to remove existing app directory: {}", e))?;
        }

        // ── Create app directory and extract ──
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create app directory: {}", e))?;

        extract::extract_strip_prefix(pkg_path, &app_dir)
            .map_err(|e| format!("Failed to extract package: {}", e))?;

        let bin_path = app_dir.join(&config.bin_rel_path);
        if !bin_path.exists() {
            let _ = std::fs::remove_dir_all(&app_dir);
            return Err(format!(
                "Binary not found at expected path: {}",
                bin_path.display()
            ));
        }

        // ── Generate .desktop ──
        let desktop_path = desktop::generate_desktop(
            &app_dir,
            app_name,
            &config.bin_rel_path,
            config.icon_rel_path.as_deref(),
            config.cicon_path.as_deref(),
        )
        .map_err(|e| format!("Failed to generate .desktop: {}", e))?;

        // ── Generate .service ──
        let service_path = if config.service {
            Some(
                service::generate_service(&app_dir, app_name, &config.bin_rel_path)
                    .map_err(|e| format!("Failed to generate .service: {}", e))?,
            )
        } else {
            None
        };

        // ── Create directories for symlinks ──
        let link_dirs = [&bin_dir, &desktop_dir, &systemd_dir, &autostart_dir];
        for d in &link_dirs {
            let _ = std::fs::create_dir_all(d);
        }

        // ── Create symlinks ──
        let mut symlink_targets: Vec<PathBuf> = Vec::new();
        let mut success = true;

        let bin_link = bin_dir.join(app_name);
        symlink_targets.push(bin_link.clone());
        if !symlink::create_symlink(&bin_path, &bin_link, "bin symlink") {
            success = false;
        }

        let desktop_link = desktop_dir.join(format!("{}.desktop", app_name));
        symlink_targets.push(desktop_link.clone());
        if !symlink::create_symlink(&desktop_path, &desktop_link, "desktop symlink") {
            success = false;
        }

        let service_link = service_path.as_ref().map(|sp| {
            let sl = systemd_dir.join(format!("{}.service", app_name));
            symlink_targets.push(sl.clone());
            if !symlink::create_symlink(sp, &sl, "service symlink") {
                success = false;
            }
            sl
        });

        let autostart_link = if config.autostart {
            let al = autostart_dir.join(format!("{}.desktop", app_name));
            symlink_targets.push(al.clone());
            if !symlink::create_symlink(&desktop_path, &al, "autostart symlink") {
                success = false;
            }
            Some(al)
        } else {
            None
        };

        if !success {
            return Err(
                "Installation failed due to conflicts. Run the app's uninstall.sh to clean up."
                    .to_string(),
            );
        }

        // ── Enable service ──
        if service_path.is_some() {
            service::enable_service(app_name);
        }

        // ── Write uninstall script ──
        let uninstall_path =
            uninstall::write_uninstall_script(&app_dir, app_name, &symlink_targets, config.service)
                .map_err(|e| format!("Failed to write uninstall script: {}", e))?;

        // ── Check PATH ──
        if !paths::is_in_path(&bin_dir) {
            eprintln!(
                "[!] Warning: {} is not in your PATH.",
                bin_dir.display()
            );
            eprintln!("    Add this to your shell config: export PATH=\"$HOME/.local/bin:$PATH\"");
        }

        Ok(InstallResult {
            app_dir,
            bin_link,
            desktop_link,
            service_link,
            autostart_link,
            uninstall_path,
        })
    }
}