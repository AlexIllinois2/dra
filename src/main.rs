// NOTE: this rule is not supported by rust-analyzer or JetBrains Rust plugin go to definition/refactoring tools so disable it until it's supported properly
#![allow(clippy::uninlined_format_args)]

use crate::cli::color::Color;
use crate::cli::completion_handler::CompletionHandler;
use crate::cli::download_handler::DownloadHandler;
use crate::cli::install_app_handler::InstallAppHandler;
use crate::cli::result::{HandlerError, HandlerResult};
use crate::cli::root_command::{Cli, Command};
use crate::cli::untag_handler::UntagHandler;
use crate::github::proxy::{PrefixMode, ProxyConfig};
use clap::Parser;
use std::process::exit;

mod cli;
mod env_var;
mod github;
mod installer;
mod registry;
mod system;
mod temp_file;
mod vector;

fn main() {
    let cli: Cli = Cli::parse();
    init_ctrl_c_handler();
    handle(run(cli));
}

// NOTE: this is needed to restore the cursor if CTRL+C is
// pressed during the asset selection (https://github.com/mitsuhiko/dialoguer/issues/77)
fn init_ctrl_c_handler() {
    ctrlc::set_handler(move || {
        let term = dialoguer::console::Term::stderr();
        let _ = term.show_cursor();
        exit(1);
    })
    .expect("Error initializing CTRL+C handler")
}

fn run(cli: Cli) -> HandlerResult {
    match cli.cmd {
        Command::Download {
            repo,
            select,
            automatic,
            tag,
            output,
            install,
            install_file,
            asset_prefix,
            asset_prefix_mode,
            api_prefix,
            api_prefix_mode,
        } => {
            DownloadHandler::new(
                repo,
                select,
                automatic,
                tag,
                output,
                install,
                install_file,
                asset_prefix,
                asset_prefix_mode,
                api_prefix,
                api_prefix_mode,
            )
            .run()
        }
        Command::Untag {
            repo,
            asset_prefix,
            asset_prefix_mode,
            api_prefix,
            api_prefix_mode,
        } => {
            let proxy = proxy_from_cli_args(
                asset_prefix,
                asset_prefix_mode,
                api_prefix,
                api_prefix_mode,
            );
            UntagHandler::new(repo, proxy).run()
        }
        Command::Completion { shell } => CompletionHandler::new(shell).run(),
        Command::InstallApp {
            repo,
            pkg,
            bin,
            name,
            icon,
            cicon,
            service,
            autostart,
            yes,
            select,
            automatic,
            tag,
            app_root,
        } => InstallAppHandler::new(
            repo,
            pkg,
            bin,
            name,
            icon,
            cicon,
            service,
            autostart,
            yes,
            select,
            automatic,
            tag,
            app_root,
        )
        .run(),
        Command::UninstallApp { name } => {
            crate::cli::install_app_handler::handle_uninstall_app(&name)
        }
        Command::Remove { name } => crate::cli::remove_handler::handle_remove(&name),
        Command::List => crate::cli::list_handler::handle_list(),
        Command::Update { name } => crate::cli::update_handler::handle_update(name),
    }
}

/// 从 CLI 参数构造 ProxyConfig
fn proxy_from_cli_args(
    asset_prefix: Option<String>,
    asset_prefix_mode: Option<String>,
    api_prefix: Option<String>,
    api_prefix_mode: Option<String>,
) -> ProxyConfig {
    ProxyConfig {
        asset_prefix,
        asset_mode: match asset_prefix_mode.as_deref().map(|x| x.to_lowercase()).as_deref() {
            Some("replace-host") | Some("replace_host") => PrefixMode::ReplaceHost,
            _ => PrefixMode::Prepend,
        },
        api_prefix,
        api_mode: match api_prefix_mode.as_deref().map(|x| x.to_lowercase()).as_deref() {
            Some("replace-host") | Some("replace_host") => PrefixMode::ReplaceHost,
            _ => PrefixMode::Prepend,
        },
    }
}

fn handle(result: HandlerResult) {
    if let Err(error) = result {
        match error {
            HandlerError::Default(msg) => {
                eprintln!("{}", Color::new(&msg).red().bold());
                exit(1)
            }
            HandlerError::OperationCancelled(msg) => {
                println!("Operation cancelled: {}", Color::new(&msg).bold());
            }
        }
    }
}
