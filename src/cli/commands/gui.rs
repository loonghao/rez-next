//! rez gui - Launch rez GUI (HTML-based status report generation)
//!
//! In rez-next, we implement this as an HTML report generator
//! since we don't have a Qt/PySide dependency.

use rez_next_common::config::RezCoreConfig;
use std::path::PathBuf;

/// Arguments for the gui command
#[derive(Debug, Default)]
pub struct GuiArgs {
    /// Output HTML file path (default: rez-status.html)
    pub output: Option<String>,
    /// Open in browser after generating
    pub open: bool,
    /// Filter to specific package family
    pub package: Option<String>,
}

/// Package info collected for display
struct PkgInfo {
    repo: String,
    name: String,
    version: String,
    description: String,
}

/// Execute the gui command
pub async fn execute(args: &GuiArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = RezCoreConfig::load();
    let output_path = args
        .output
        .as_deref()
        .unwrap_or("rez-status.html")
        .to_string();

    eprintln!("Generating rez status report...");

    let mut all_packages: Vec<PkgInfo> = Vec::new();
    let mut all_paths = config.packages_path.clone();
    all_paths.push(config.local_packages_path.clone());

    for repo_path in &all_paths {
        let path = std::path::Path::new(repo_path);
        if !path.exists() {
            continue;
        }
        // Scan directory: <repo>/<name>/<version>/
        if let Ok(families) = std::fs::read_dir(path) {
            for family_entry in families.flatten() {
                let family_name = family_entry.file_name().to_string_lossy().to_string();

                // Apply package filter
                if let Some(ref filter) = args.package {
                    if &family_name != filter {
                        continue;
                    }
                }

                let family_path = family_entry.path();
                if !family_path.is_dir() {
                    continue;
                }

                if let Ok(versions) = std::fs::read_dir(&family_path) {
                    for ver_entry in versions.flatten() {
                        let ver_name = ver_entry.file_name().to_string_lossy().to_string();
                        let ver_path = ver_entry.path();
                        if !ver_path.is_dir() {
                            continue;
                        }

                        // Try to read description from package.py or package.yaml
                        let description = read_description(&ver_path);

                        all_packages.push(PkgInfo {
                            repo: repo_path.clone(),
                            name: family_name.clone(),
                            version: ver_name,
                            description,
                        });
                    }
                }
            }
        }
    }

    // Sort by name then version
    all_packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));

    let html = generate_html_report(&config, &all_packages);
    std::fs::write(&output_path, &html)?;
    eprintln!("Status report written to: {}", output_path);

    if args.open {
        open_in_browser(&output_path);
    }

    Ok(())
}

/// Try to read package description from package.py or package.yaml
fn read_description(pkg_dir: &std::path::Path) -> String {
    // Try package.py
    let py_path = pkg_dir.join("package.py");
    if py_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&py_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("description") && trimmed.contains('=') {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        let s = val.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !s.is_empty() {
                            return s;
                        }
                    }
                }
            }
        }
    }

    // Try package.yaml
    let yaml_path = pkg_dir.join("package.yaml");
    if yaml_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&yaml_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("description:") {
                    let val = trimmed.trim_start_matches("description:").trim();
                    if !val.is_empty() {
                        return val.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }
    }

    String::new()
}

/// Generate an HTML status report
fn generate_html_report(config: &RezCoreConfig, packages: &[PkgInfo]) -> String {
    let package_rows: String = packages
        .iter()
        .map(|p| {
            format!(
                r#"<tr>
  <td class="name">{}</td>
  <td class="version">{}</td>
  <td class="repo">{}</td>
  <td class="desc">{}</td>
</tr>"#,
                html_escape(&p.name),
                html_escape(&p.version),
                html_escape(&p.repo),
                html_escape(&p.description),
            )
        })
        .collect();

    let repo_list: String = {
        let mut paths = config.packages_path.clone();
        paths.push(config.local_packages_path.clone());
        paths
            .iter()
            .map(|p| format!("<li><code>{}</code></li>", html_escape(p)))
            .collect()
    };

    let total = packages.len();
    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>rez-next Status Report</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; background: #0f172a; color: #e2e8f0; }}
    .header {{ background: linear-gradient(135deg, #1e40af, #7c3aed); padding: 32px; }}
    .header h1 {{ margin: 0; font-size: 28px; color: #fff; }}
    .header p {{ margin: 8px 0 0; opacity: 0.8; color: #c7d2fe; }}
    .container {{ max-width: 1200px; margin: 0 auto; padding: 32px 24px; }}
    .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 32px; }}
    .stat-card {{ background: #1e293b; border-radius: 12px; padding: 20px; border: 1px solid #334155; }}
    .stat-card .value {{ font-size: 36px; font-weight: 700; color: #60a5fa; }}
    .stat-card .label {{ font-size: 14px; color: #94a3b8; margin-top: 4px; }}
    .section {{ margin-bottom: 32px; }}
    .section h2 {{ font-size: 20px; color: #e2e8f0; margin: 0 0 16px; padding-bottom: 8px; border-bottom: 1px solid #334155; }}
    .repos {{ list-style: none; padding: 0; margin: 0; }}
    .repos li {{ background: #1e293b; border-radius: 8px; padding: 10px 16px; margin-bottom: 8px; border: 1px solid #334155; }}
    table {{ width: 100%; border-collapse: collapse; background: #1e293b; border-radius: 12px; overflow: hidden; border: 1px solid #334155; }}
    th {{ background: #0f172a; padding: 12px 16px; text-align: left; font-size: 13px; font-weight: 600; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; }}
    td {{ padding: 12px 16px; border-top: 1px solid #334155; font-size: 14px; }}
    td.name {{ color: #60a5fa; font-weight: 600; }}
    td.version {{ color: #34d399; font-family: monospace; }}
    td.repo {{ color: #94a3b8; font-size: 12px; }}
    tr:hover td {{ background: #263145; }}
    .empty {{ text-align: center; padding: 48px; color: #64748b; }}
    .timestamp {{ font-size: 12px; color: #475569; text-align: right; margin-top: 24px; }}
  </style>
</head>
<body>
  <div class="header">
    <h1>rez-next Status Report</h1>
    <p>Package repository overview</p>
  </div>
  <div class="container">
    <div class="stats">
      <div class="stat-card">
        <div class="value">{total}</div>
        <div class="label">Total Packages</div>
      </div>
    </div>
    <div class="section">
      <h2>Package Repositories</h2>
      <ul class="repos">
        {repo_list}
      </ul>
    </div>
    <div class="section">
      <h2>Packages</h2>
      {table_content}
    </div>
    <div class="timestamp">Generated: {timestamp}</div>
  </div>
</body>
</html>"#,
        total = total,
        repo_list = repo_list,
        table_content = if packages.is_empty() {
            r#"<div class="empty">No packages found in repositories.</div>"#.to_string()
        } else {
            format!(
                r#"<table>
  <thead>
    <tr><th>Package</th><th>Version</th><th>Repository</th><th>Description</th></tr>
  </thead>
  <tbody>
    {}
  </tbody>
</table>"#,
                package_rows
            )
        },
        timestamp = timestamp,
    )
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Open file in default browser
fn open_in_browser(path: &str) {
    let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    let url = format!("file://{}", abs_path.display());

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", &url])
        .spawn();

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();

    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_generate_html_report_empty() {
        let config = RezCoreConfig::default();
        let html = generate_html_report(&config, &[]);
        assert!(html.contains("No packages found"));
        assert!(html.contains("rez-next Status Report"));
    }

    #[test]
    fn test_generate_html_report_with_packages() {
        let config = RezCoreConfig::default();
        let packages = vec![
            PkgInfo {
                repo: "/repo/local".to_string(),
                name: "python".to_string(),
                version: "3.10.0".to_string(),
                description: "Python interpreter".to_string(),
            },
            PkgInfo {
                repo: "/repo/release".to_string(),
                name: "maya".to_string(),
                version: "2024.0".to_string(),
                description: "Maya DCC".to_string(),
            },
        ];
        let html = generate_html_report(&config, &packages);
        assert!(html.contains("python"));
        assert!(html.contains("3.10.0"));
        assert!(html.contains("maya"));
        assert!(html.contains("2024.0"));
    }

    #[test]
    fn test_gui_args_default() {
        let args = GuiArgs::default();
        assert!(args.output.is_none());
        assert!(!args.open);
        assert!(args.package.is_none());
    }

    #[test]
    fn test_read_description_missing_dir() {
        let result = read_description(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_empty());
    }
}
