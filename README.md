# DRA - Download Release Assets from GitHub

[![CI](https://github.com/devmatteini/dra/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/devmatteini/dra/actions/workflows/ci.yml)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/devmatteini/dra)

A command line tool to download release assets from GitHub.

[Why should I use dra?](#why-should-i-use-dra) •
[Installation](#installation) •
[Usage](#usage) •
[Portable App Installation](#portable-app-installation) •
[Lifecycle Management](#lifecycle-management) •
[Contributing](#contributing) •
[License](#license) •
[加速下载](#下载代理)

![dra demo](./assets/demo.gif)

## Why should I use dra?

You can do everything `dra` does with the official [GitHub cli](https://cli.github.com/).

`dra` helps you download release assets more easily:

- No authentication for public repository (you cannot use `gh` without authentication)
- [Built-in generation of pattern](#non-interactive-download) to select an asset to download
  (with `gh` you need to provide [glob pattern](https://cli.github.com/manual/gh_release_download) that you need to
  create manually).
- [Automatically select and download](#automatic) an asset based on your operating system and architecture
- [Download and install assets](#install-assets), with support for the most common formats (e.g. tar/zip archives,
  deb/rpm packages...)
- [Portable App Installation](#portable-app-installation) — download GUI applications and install them with .desktop shortcuts,
  systemd services, and autostart support
- [Lifecycle Management](#lifecycle-management) — `list`, `remove`, and `update` installed applications from a local registry

## Installation

`dra` is available on Linux (x86_64, armv6, arm64), macOS (x86_64, arm64) and Windows.

### Prebuilt binaries

Download the prebuilt versions of `dra` for all supported platforms from
the [latest release](https://github.com/devmatteini/dra/releases/latest).

You can use this `bash` script to automatically download the latest release across all supported platforms.
Replace `<DESTINATION>` with the path where you want to place dra (e.g `~/.local/bin`).
If you omit `--to` option, the default value is the current working directory.

```shell
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/devmatteini/dra/refs/heads/main/install.sh | bash -s -- --to <DESTINATION>
```

### Debian-based distributions

Download the latest `.deb` package from the [release page](https://github.com/devmatteini/dra/releases/latest) and
install it via:

```shell
sudo dpkg -i dra_x.y.z_amd64.deb # adapt version number
```

### Arch Linux

`dra` can be installed from the [community repository](https://archlinux.org/packages/extra/x86_64/dra/):

```shell
pacman -S dra
```

### macOS/Linux with Homebrew

`dra` can be installed from [Homebrew](https://formulae.brew.sh/formula/dra#default):

```shell
brew install dra
```

### From source

```shell
git clone https://github.com/devmatteini/dra && cd dra
make release
./target/release/dra --version
```

### Update dra

The method to update `dra` depends on how you initially installed it.

**If you used a package manager** (e.g [Homebrew](#macoslinux-with-homebrew) or [pacman](#arch-linux)), use the
corresponding package manager commands to update `dra`.

**If you downloaded a prebuilt binary** from GitHub Releases, you have two options:

#### Option 1: Use dra to update itself

- Linux (_Note that you can't replace a binary while it's executing_)
  ```shell
  dra download -a -i -o dra-new devmatteini/dra && mv dra-new /path/to/dra
  ```
- macOS
  ```shell
  dra download -a -i -o /path/to/dra devmatteini/dra
  ```
- Windows (_Note that you can't replace a binary while it's executing_)
  ```shell
  dra download -a -i -o dra-new.exe devmatteini/dra && mv dra-new.exe /path/to/dra.exe
  ```

#### Option 2: Use the automated bash script

Follow the installation instructions on how to use the [automated bash script](#prebuilt-binaries)

## Usage

- [Download assets with interactive mode](#interactive-download)
- [Download assets with non-interactive mode](#non-interactive-download)
- [Download options](#download-options)
- [Install assets](#install-assets)
- [Portable App Installation](#portable-app-installation)
- [Lifecycle Management](#lifecycle-management)
- [Authentication](#authentication)
- [Shell completion](#shell-completion)
- [Examples](#examples)

### Interactive download

Manually select and download an asset from a repository

```shell
dra download devmatteini/dra-tests
```

### Non-Interactive download

This mode is useful to be used in automated scripts.

There are two modes to download assets: [automatic](#automatic) and [selection](#selection).

#### Automatic

Automatically select and download an asset based on your operating system and architecture

```shell
# you can use -a or --automatic
dra download -a devmatteini/dra-tests
```

> [!IMPORTANT]
> Since there is no naming convention for release assets,
> be aware that this mode may fail if no asset matches your system based on `dra` rules for recognizing an asset.

#### Selection

Select and download the first asset that matches a given pattern

```shell
dra download --select <PATTERN> devmatteini/dra-tests
```

You can pass one of the following patterns:

##### Literal

The exact name of the asset.

```shell
dra download --select helloworld.tar.gz devmatteini/dra-tests
```

##### Untagged

A version-free pattern of your asset. Most of the time, this is the pattern you will use.

It's useful when release assets contain the release tag in their name, and you want a stable pattern to download across
different releases.

First, you need to generate an untagged asset name:

```shell
dra untag devmatteini/dra-tests
# Output: helloworld_{tag}.tar.gz
```

Copy the output and run:

```shell
dra download --select "helloworld_{tag}.tar.gz" devmatteini/dra-tests
```

##### Wildcard

A [wildcard pattern](https://en.wikipedia.org/wiki/Matching_wildcards), using `*` and/or `?` special characters.

It's useful when release assets contain sections that change for each release (e.g. a build number or timestamp).

```shell
dra download --select 'helloworld*_amd64.deb' devmatteini/dra-tests
```

### Download options

All `dra-download` options works with both interactive and non-interactive modes.

Select and download an asset to custom path

```shell
dra download --output /tmp/dra-example devmatteini/dra-tests

# or save to custom directory path
dra download --output ~/Downloads devmatteini/dra-tests
```

Select and download an asset from a specific release

```shell
dra download --tag 0.1.1 devmatteini/dra-tests
```

Select and download source code archives

```shell
dra download devmatteini/dra-tests
Release tag is 0.1.5
? Pick the asset to download ›
  helloworld_0.1.5.tar.gz
❯ Source code (tar.gz)
  Source code (zip)
```

### Install assets

Download and install an asset (on both interactive and non-interactive modes)

```shell
dra download --install devmatteini/dra-tests
```

Supported assets that can be installed are:

- Debian packages (requires elevated privileges)
- RPM packages (requires elevated privileges)
- Tar archives with executable(s)
- Zip files with executable(s)
- 7-Zip files with executable(s) (requires `7z` cli to be installed and in your `PATH`)
- Compressed executable files
- Executable files
- AppImage files

You can use `-I/--install-file <INSTALL_FILE>` option when a tar/zip archive contains many executables or
when `dra` can't automatically detect which one to install:

```shell
dra download -s helloworld-many-executables-unix.tar.gz -I helloworld-v2 devmatteini/dra-tests
```

You can also specify this option multiple times to install multiples executables

```shell
dra download -s helloworld-many-executables-unix.tar.gz -I helloworld-v2 -I random-script devmatteini/dra-tests
```

### Portable App Installation

Download and install a portable GUI application from a GitHub release asset (tar.gz) into `~/.local/app/<name>/`.

This automatically generates `.desktop` shortcuts, optional systemd user services, bin symlinks, and an uninstall script.

```shell
# Download and install a portable app (e.g. VS Code)
dra install-app microsoft/vscode --bin bin/code --name vscode

# With custom icon and systemd service
dra install-app microsoft/vscode -s '*x64.tar.gz' --bin bin/code --name vscode \
  --icon share/icons/code.png --service --autostart

# Install from a local tar.gz package (skip GitHub download)
dra install-app --pkg ./code-server.tar.gz --bin bin/code --name code-server

# Skip confirmation if app directory already exists
dra install-app --pkg ./local.tar.gz --bin bin/code --name myapp --yes
```

**What gets installed:**

| Path | Description |
|---|---|
| `~/.local/app/<name>/` | Application directory (extracted contents) |
| `~/.local/bin/<name>` | Symlink to the executable |
| `~/.local/share/applications/<name>.desktop` | Desktop shortcut (app menu) |
| `~/.config/systemd/user/<name>.service` | Systemd user service (with `--service`) |
| `~/.config/autostart/<name>.desktop` | Autostart entry (with `--autostart`) |
| `~/.local/app/<name>/uninstall.sh` | Uninstall script |

### Lifecycle Management

All installed applications (both `dra download --install` and `dra install-app`) are automatically recorded in a local registry at `~/.local/share/dra/installed_apps.json`. This enables lifecycle management commands.

#### List installed applications

```shell
dra list
```

Example output:

```
Name                 Version        Type             Repository
--------------------------------------------------------------------------------
ripgrep              13.0.0         ArchiveBin        BurntSushi/ripgrep
vscode               1.85.0         PortableApp       microsoft/vscode
```

#### Remove an installed application

```shell
# Remove a tool installed via `dra download --install`
dra remove ripgrep
# → Removes ~/.local/bin/rg and clears the registry entry

# Remove a portable app installed via `dra install-app`
dra remove vscode
# → Removes ~/.local/app/vscode, symlinks, .desktop, .service, and registry entry
```

The `remove` command handles all install types:

| Install Type | Cleanup Behavior |
|---|---|
| `Bin` / `ArchiveBin` / `AppImage` | Deletes executables from `~/.local/bin/` |
| `PortableApp` | Removes app directory, symlinks, `.desktop`, `.service`, autostart |

#### Update installed applications

```shell
# Update a specific app
dra update ripgrep

# Check all installed apps for updates
dra update
```

The `update` command:

1. Reads the registry to find installed applications and their versions
2. Queries GitHub for the latest release tag
3. Compares versions (normalized, ignoring `v` prefix)
4. Downloads and installs the new version
5. Updates the registry with the new version

> [!NOTE]
> Currently `dra update` supports `Bin`, `ArchiveBin`, and `AppImage` types. For `PortableApp` type, use `dra install-app` with the latest version instead.

### Authentication

In order to download assets from private repositories and avoid rate limit
issues ([60 requests per hour](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api?apiVersion=2022-11-28#primary-rate-limit-for-unauthenticated-users)
is the default for unauthenticated users), `dra` must make authenticated requests to GitHub.

You can create
a [personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)
and then export one of the following environment variables:

1. `DRA_GITHUB_TOKEN`
2. `GITHUB_TOKEN`
3. `GH_TOKEN`

If none of the above environment variables are set,
the [GitHub cli token](https://cli.github.com/manual/gh_auth_token) (if available) will be used as default value.
You need to install [GitHub cli](https://cli.github.com/) and then run `gh auth login`.

#### Disable authentication

If you would like to disable GitHub authentication, you can export the environment variable
`DRA_DISABLE_GITHUB_AUTHENTICATION=true`

### 下载代理

如果所在网络无法直接访问 GitHub，可以通过环境变量或 CLI 参数配置下载前缀，让 `dra` 走代理/镜像下载资源。

支持两种前缀模式：

- **prepend**（默认）：在原 URL 前完整拼接前缀。适用于 `https://gitproxy.com/https://github.com/...` 这类代理。
- **replace-host**：剥掉原 URL 的 `https://github.com/` 再拼接前缀。适用于 `https://xget.xi-xu.me/gh/foo/bar/...` 这类镜像。

下载前缀拆成两组独立配置：

| 配置 | 对应域名 | 作用 |
|---|---|---|
| `asset-prefix` | `github.com`, `api.github.com` | 覆盖实际文件下载和源码包（tarball/zipball）URL |
| `api-prefix` | `api.github.com` | 覆盖 API 请求（拉取 release 元数据）URL |

仅在需要为 API 和下载流量配置不同镜像时才需设置 `api-prefix`；大多数场景下只配 `asset-prefix` 即可。

**环境变量方式：**

```shell
# gitproxy（prepend 模式，默认）
export DRA_ASSET_PREFIX="https://gitproxy.com"
dra download -a devmatteini/dra-tests

# xget（replace-host 模式）
export DRA_ASSET_PREFIX="https://xget.xi-xu.me/gh"
export DRA_ASSET_PREFIX_MODE="replace-host"
dra download -a devmatteini/dra-tests

# 同时给 API 配镜像
export DRA_API_PREFIX="https://gh.api.proxy"
export DRA_API_PREFIX_MODE="replace-host"
dra download devmatteini/dra-tests
```

**CLI 参数方式（临时覆盖，优先级高于环境变量）：**

```shell
dra download --asset-prefix https://gitproxy.com devmatteini/dra-tests
dra download --asset-prefix https://xget.xi-xu.me/gh --asset-prefix-mode replace-host devmatteini/dra-tests
```

> [!NOTE]
> - `asset-prefix` 同时也改写源码包（tarball/zipball）URL，纯下载镜像（如 xget 的 `/gh/`）可能不支持 API 域名的源码包。若遇到源码包下载失败，可额外配置 `DRA_API_PREFIX`。
> - 部分代理可能因携带 `Authorization` 头而报 400。可用 `DRA_DISABLE_GITHUB_AUTHENTICATION=1` 关闭认证后重试。

### Shell completion

Generate shell completion

```shell
dra completion bash > dra-completion
source dra-completion
```

See all supported shell with `dra completion -h`

### Examples

Install an executable from a tar archive

```shell
dra download -s helloworld.tar.gz -i devmatteini/dra-tests
./helloworld
```

Install and move the executable to a custom directory

```shell
dra download -a -i -o ~/.local/bin/ devmatteini/dra-tests
~/.local/bin/helloworld
```

Install an executable file

```shell
dra download -s helloworld-unix -i devmatteini/dra-tests
./helloworld-unix
```

Install an executable from a compressed file

```shell
dra download -s helloworld-compressed-unix.bz2 -i devmatteini/dra-tests
./helloworld-compressed-unix
```

Install and rename the executable (useful when downloading an executable or compressed file)

```shell
dra download -s helloworld-unix -i -o helloworld devmatteini/dra-tests
./helloworld
```

Install a specific executable when many are available

```shell
dra download -s helloworld-many-executables-unix.tar.gz -I helloworld-v2 devmatteini/dra-tests
./helloworld-v2
```

Install multiple executables from a tar/zip archive

```shell
dra download -s helloworld-many-executables-unix.tar.gz -I helloworld-v2 -I random-script devmatteini/dra-tests
./helloworld-v2
./random-script
```

---

For more information on args/flags/options/commands run:

```shell
dra --help
dra <command> --help
```

## Contributing

Take a look at the [CONTRIBUTING.md](CONTRIBUTING.md) guidelines.

### Found a Bug?

If you find a bug in the source code, you can help us
by [submitting an issue](https://github.com/devmatteini/dra/issues/new) to our GitHub Repository. Even
better, you can submit a Pull Request with a fix.

### Missing a Feature?

You can request a new feature
by [submitting a discussion](https://github.com/devmatteini/dra/discussions/new/choose) to
our GitHub Repository.
If you would like to implement a new feature, please consider the size of the change and reach out to
better coordinate our efforts and prevent duplication of work.

## License

`dra` is made available under the terms of the MIT License.

See the [LICENSE](LICENSE) file for license details.
