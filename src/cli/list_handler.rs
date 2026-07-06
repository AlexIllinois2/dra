use crate::cli::color::Color;
use crate::cli::result::HandlerResult;
use crate::registry;
use crate::registry::app_record::InstallType;

pub fn handle_list() -> HandlerResult {
    let registry = registry::Registry::load();
    let records = registry.list_records();

    if records.is_empty() {
        println!("{}", Color::new("No installed apps found.").bold());
        println!("Run 'dra download --install <repo>' or 'dra install-app <repo>' to install something.");
        return Ok(());
    }

    // Header
    println!(
        "{:<20} {:<14} {:<16} {:<30}",
        Color::new("Name").bold(),
        Color::new("Version").bold(),
        Color::new("Type").bold(),
        Color::new("Repository").bold(),
    );
    println!("{}", "-".repeat(80));

    for record in records {
        let type_str = match record.install_type {
            InstallType::Bin => "Bin",
            InstallType::ArchiveBin => "ArchiveBin",
            InstallType::PortableApp => "PortableApp",
            InstallType::AppImage => "AppImage",
        };

        println!(
            "{:<20} {:<14} {:<16} {:<30}",
            record.name,
            record.installed_version,
            type_str,
            record.repo,
        );
    }

    println!();
    println!(
        "Total: {} app(s) installed.",
        Color::new(&records.len().to_string()).bold()
    );

    Ok(())
}