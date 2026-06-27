use crate::env_var;
use crate::github::constants::{
    DRA_API_PREFIX, DRA_API_PREFIX_MODE, DRA_ASSET_PREFIX, DRA_ASSET_PREFIX_MODE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixMode {
    Prepend,     // 保留完整原 URL，直接前缀拼接
    ReplaceHost, // 剥掉 host 后拼接
}

impl Default for PrefixMode {
    fn default() -> Self {
        PrefixMode::Prepend
    }
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub asset_prefix: Option<String>,
    pub asset_mode: PrefixMode,
    pub api_prefix: Option<String>,
    pub api_mode: PrefixMode,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            asset_prefix: None,
            asset_mode: PrefixMode::default(),
            api_prefix: None,
            api_mode: PrefixMode::default(),
        }
    }
}

impl ProxyConfig {
    pub fn from_environment() -> Self {
        Self {
            asset_prefix: env_var::string(DRA_ASSET_PREFIX),
            asset_mode: parse_mode(env_var::string(DRA_ASSET_PREFIX_MODE)),
            api_prefix: env_var::string(DRA_API_PREFIX),
            api_mode: parse_mode(env_var::string(DRA_API_PREFIX_MODE)),
        }
    }

    /// 对 asset 下载 URL 进行改写。
    /// 同时识别 https://github.com/ 与 https://api.github.com/ 两个 host，
    /// 确保源码包（tarball/zipball）URL 也能被 asset 前缀命中。
    pub fn rewrite_asset(&self, url: &str) -> String {
        rewrite_url(url, self.asset_prefix.as_deref(), self.asset_mode, &[
            "https://github.com/",
            "https://api.github.com/",
        ])
    }

    /// 对 API URL 进行改写。
    pub fn rewrite_api(&self, url: &str) -> String {
        rewrite_url(url, self.api_prefix.as_deref(), self.api_mode, &[
            "https://api.github.com/",
        ])
    }

    pub fn has_any_prefix(&self) -> bool {
        self.asset_prefix.is_some() || self.api_prefix.is_some()
    }
}

fn parse_mode(s: Option<String>) -> PrefixMode {
    match s.as_deref().map(|x| x.to_lowercase()).as_deref() {
        Some("replace-host") | Some("replace_host") => PrefixMode::ReplaceHost,
        _ => PrefixMode::Prepend,
    }
}

/// 核心改写函数。
/// - prefix: 可选的前缀，为 None 时返回原 URL
/// - mode: Prepend 或 ReplaceHost
/// - hosts: 在 ReplaceHost 模式下依次尝试剥除的 host 列表
fn rewrite_url(url: &str, prefix: Option<&str>, mode: PrefixMode, hosts: &[&str]) -> String {
    let Some(prefix) = prefix else {
        return url.to_string();
    };

    // 规整：去除末尾 /，之后统一用 format!("{}/{}", prefix, rest) 拼接
    let prefix = prefix.trim_end_matches('/');

    match mode {
        PrefixMode::Prepend => format!("{}/{}", prefix, url),
        PrefixMode::ReplaceHost => {
            for host in hosts {
                if let Some(rest) = url.strip_prefix(host) {
                    return format!("{}/{}", prefix, rest);
                }
            }
            url.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepend_mode_keeps_full_url() {
        let cfg = ProxyConfig {
            asset_prefix: Some("https://gitproxy.com".into()),
            asset_mode: PrefixMode::Prepend,
            ..Default::default()
        };
        assert_eq!(
            cfg.rewrite_asset("https://github.com/foo/bar/releases/download/v1/a.tar.gz"),
            "https://gitproxy.com/https://github.com/foo/bar/releases/download/v1/a.tar.gz"
        );
    }

    #[test]
    fn replace_host_strips_github_prefix() {
        let cfg = ProxyConfig {
            asset_prefix: Some("https://xget.xi-xu.me/gh".into()),
            asset_mode: PrefixMode::ReplaceHost,
            ..Default::default()
        };
        assert_eq!(
            cfg.rewrite_asset("https://github.com/foo/bar/releases/download/v1/a.tar.gz"),
            "https://xget.xi-xu.me/gh/foo/bar/releases/download/v1/a.tar.gz"
        );
    }

    #[test]
    fn no_prefix_returns_original() {
        let cfg = ProxyConfig::default();
        let url = "https://github.com/foo/bar";
        assert_eq!(cfg.rewrite_asset(url), url);
    }

    #[test]
    fn trailing_slash_normalized() {
        let cfg = ProxyConfig {
            asset_prefix: Some("https://gitproxy.com/".into()),
            asset_mode: PrefixMode::Prepend,
            ..Default::default()
        };
        assert_eq!(
            cfg.rewrite_asset("https://github.com/foo/bar"),
            "https://gitproxy.com/https://github.com/foo/bar"
        );
    }

    #[test]
    fn api_rewrite_only_matches_api_host() {
        let cfg = ProxyConfig {
            api_prefix: Some("https://gh.api.proxy".into()),
            api_mode: PrefixMode::ReplaceHost,
            ..Default::default()
        };
        assert_eq!(
            cfg.rewrite_api("https://api.github.com/repos/foo/bar/releases/latest"),
            "https://gh.api.proxy/repos/foo/bar/releases/latest"
        );
    }

    #[test]
    fn replace_host_multi_host_asset_rewrite() {
        // rewrite_asset 应同时识别 github.com 和 api.github.com
        let cfg = ProxyConfig {
            asset_prefix: Some("https://xget.xi-xu.me/gh".into()),
            asset_mode: PrefixMode::ReplaceHost,
            ..Default::default()
        };
        assert_eq!(
            cfg.rewrite_asset("https://api.github.com/repos/foo/foo/tarball/v1"),
            "https://xget.xi-xu.me/gh/repos/foo/foo/tarball/v1"
        );
    }

    #[test]
    fn replace_host_unknown_host_untouched() {
        let cfg = ProxyConfig {
            asset_prefix: Some("https://proxy".into()),
            asset_mode: PrefixMode::ReplaceHost,
            ..Default::default()
        };
        let url = "https://other.com/path";
        assert_eq!(cfg.rewrite_asset(url), url);
    }

    #[test]
    fn parse_mode_defaults_to_prepend() {
        assert_eq!(parse_mode(None), PrefixMode::Prepend);
        assert_eq!(
            parse_mode(Some("invalid".into())),
            PrefixMode::Prepend
        );
    }

    #[test]
    fn parse_mode_variants() {
        assert_eq!(
            parse_mode(Some("replace-host".into())),
            PrefixMode::ReplaceHost
        );
        assert_eq!(
            parse_mode(Some("replace_host".into())),
            PrefixMode::ReplaceHost
        );
        assert_eq!(
            parse_mode(Some("REPLACE-HOST".into())),
            PrefixMode::ReplaceHost
        );
    }

    #[test]
    fn has_any_prefix_true_when_asset_set() {
        let cfg = ProxyConfig {
            asset_prefix: Some("https://proxy".into()),
            ..Default::default()
        };
        assert!(cfg.has_any_prefix());
    }

    #[test]
    fn has_any_prefix_true_when_api_set() {
        let cfg = ProxyConfig {
            api_prefix: Some("https://proxy".into()),
            ..Default::default()
        };
        assert!(cfg.has_any_prefix());
    }

    #[test]
    fn has_any_prefix_false_when_none() {
        let cfg = ProxyConfig::default();
        assert!(!cfg.has_any_prefix());
    }
}