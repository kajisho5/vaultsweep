# vaultsweep

**English** · [日本語](README.ja.md)

vaultsweep scans your 1Password vault, ranks risky logins, and walks you through changing passwords and closing accounts you no longer use. Free and open source, for Windows and Mac.

[Website](https://kajisho5.github.io/vaultsweep/) · [Download](https://github.com/kajisho5/vaultsweep/releases/latest) · [Sponsor](https://github.com/sponsors/kajisho5)

> vaultsweep is not made by 1Password. It runs on your own computer through the official 1Password CLI.

## What it does

- **Finds 8 kinds of problems**: leaked passwords, logins on breached sites, reused passwords, near duplicates (one or two characters apart), weak passwords, 2FA available but off, logins untouched for a long time, and duplicate items. Everything is ranked with leaks first.
- **Helps you change passwords**, one at a time or in a batch. The new password is generated and saved inside 1Password. If the site rejects it, one click restores the old one.
- **Follows each site's password rules.** Rules such as "no symbols", "20 characters max" or "digits only" come from Apple's public data (about 430 sites). The new password is checked against them, and you can set your own rules per site.
- **Helps you close accounts**: replace personal details with dummies, cancel, keep a record, then move the item to the 1Password Archive.
- **Scans on a schedule** (daily, weekly or monthly) and notifies you only about new problems.
- **Works with AI apps over MCP.** Ask Claude "which of my passwords leaked?" The AI gets scores, counts and site names, never passwords.
- **Updates itself.** New versions are checked against our signature before installing.
- English and Japanese, light and dark themes.

## Safety rules

- Passwords, usernames and notes are never shown, stored or sent to AI tools.
- New passwords are created by `op` (1Password CLI) inside 1Password. When a password is restored, the value goes through standard input, never as a command argument.
- The leak check sends only the first 5 characters of a password's SHA-1 hash (Have I Been Pwned k-anonymity), with Add-Padding so the response size reveals nothing.
- Copied passwords clear after 30 seconds and are kept out of clipboard history and cloud clipboard (Windows) and marked as concealed (Mac).
- Nothing is permanently deleted. Cleanup only moves items to the 1Password Archive, so you can always restore them.
- AI apps can only *request* a change or a cleanup. Nothing happens until you approve it in the app.
- The app only contacts the sources listed under [Data sources](#data-sources), plus GitHub for update checks. Offline mode turns all of that off.

## Getting started

1. Open the 1Password app, unlock it, and turn on Windows Hello or Touch ID.
2. In 1Password, go to Settings → Developer and turn on **Integrate with 1Password CLI**.
3. Install the 1Password CLI ([official guide](https://www.1password.dev/cli/get-started)):
   - Windows (PowerShell): `winget install 1password-cli`
   - Mac (Homebrew): `brew install 1password-cli`
4. Download vaultsweep from [Releases](https://github.com/kajisho5/vaultsweep/releases/latest):
   - Windows 10 and 11: `vaultsweep_<version>_x64-setup.exe`
   - Mac (Apple silicon and Intel, macOS 11 or later): `vaultsweep-<version>.dmg`. It is signed with a Developer ID and notarized by Apple.
5. Press **Scan now** and approve the 1Password prompt.

The Windows installer is not code signed yet, so SmartScreen may show "Windows protected your PC" the first time. Choose **More info** → **Run anyway**.

## Using it from AI apps (MCP)

Settings shows the exact snippet to paste into your AI app. It looks like this (`command` is replaced with the real path):

```json
{ "mcpServers": { "vaultsweep": { "command": "C:\\...\\vaultsweep-mcp.exe", "args": ["--mcp"] } } }
```

On a Mac the command is inside the app: `/Applications/vaultsweep.app/Contents/MacOS/vaultsweep-mcp`.

Tools: `get_summary` `list_issues` `get_item_issues` `run_scan` `mark_in_use` `add_to_cancel_list` `request_password_change` `request_archive`. The server answers both the 2026-07-28 protocol (`server/discover`) and older clients that start with `initialize`.

## Data sources

| Source | Used for | License |
| --- | --- | --- |
| [Have I Been Pwned](https://haveibeenpwned.com/) | Leaked passwords, breached sites (email check needs a paid key) | CC BY 4.0 |
| [JustDeleteMe](https://github.com/jdm-contrib/jdm) | Account deletion pages and difficulty | MIT |
| [2FA Directory](https://2fa.directory/) | Sites that support two factor sign in | MIT |
| [Passkeys Directory](https://passkeys.2fa.directory/) | Sites that support passkeys | CC BY 4.0 |
| [Apple Password Manager Resources](https://github.com/apple/password-manager-resources) | Each site's password rules (a copy ships with the app) | MIT |

Data sourced from 2FA Directory by 2factorauth.

## How it is built

| Part | What it is |
| --- | --- |
| `crates/core` | The core (Rust): 1Password CLI, checks, HIBP, public lists, password rules, storage, MCP server, `vaultsweep-mcp` |
| `src-tauri` | The app shell (Tauri 2): clipboard, notifications, scheduled scans (Task Scheduler / launchd), updates |
| `src` | The UI (React, TypeScript, Tailwind CSS). Opened outside the app, it runs as a demo with fictional data |
| `site` | The website (Astro), published to GitHub Pages |

Where data is kept: `%APPDATA%\vaultsweep` on Windows, `~/Library/Application Support/vaultsweep` on Mac. Settings, the cancel list, history and the last scan result only. No secret values.

### Development

```sh
npm ci
cargo test -p vaultsweep-core     # core tests
npm run dev                        # UI only (demo) at http://localhost:5173
```

Builds run on GitHub Actions (`.github/workflows/build.yml`). When `package.json` has a version with no release yet, a push to `main` builds Windows and Mac, signs and notarizes the Mac app, and publishes the release with update files.

## Support

vaultsweep has no ads and no paid plan. If it helped you, you can [sponsor it on GitHub](https://github.com/sponsors/kajisho5). Every feature stays free either way.

## License

MIT. 1Password is a trademark of AgileBits Inc.
