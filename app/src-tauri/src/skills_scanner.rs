use crate::error::Result;
use crate::fs as appfs;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: SkillSource,
    pub plugin_name: Option<String>,
    pub path: String,
    pub installed: bool,
    pub category: Option<String>,
    /// For recommended plugins: list of skills bundled inside.
    /// `None` for individually installed skills or recommend plugins
    /// without explicit bundled skill metadata in seed.
    pub bundled_skills: Option<Vec<BundledSkillView>>,
    /// For recommended plugins: marketplace registry id to install from
    /// (e.g. "anthropic-agent-skills"). Used by frontend to build install cmd.
    pub marketplace_id: Option<String>,
    /// For recommended plugins: argument to `claude plugin marketplace add <arg>`
    /// (typically the GitHub repo path, e.g. "anthropics/skills").
    pub marketplace_add_arg: Option<String>,
    /// Human-readable label of who maintains the marketplace (e.g. "Anthropic 官方").
    pub marketplace_owner_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundledSkillView {
    pub name: String,
    pub description_zh: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillSource {
    User,
    Project,
    Plugin,
    Recommend,
}

#[derive(Debug, Deserialize)]
struct SeedBundledSkill {
    name: String,
    #[serde(rename = "descriptionZh")]
    description_zh: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SeedPluginEntry {
    #[serde(rename = "marketplaceId")]
    marketplace_id: String,
    name: String,
    description: String,
    category: Option<String>,
    #[serde(rename = "bundledSkills")]
    bundled_skills: Option<Vec<SeedBundledSkill>>,
}

#[derive(Debug, Deserialize)]
struct SeedMarketplace {
    #[serde(rename = "marketplaceAddArg")]
    marketplace_add_arg: String,
    #[serde(rename = "ownerLabel")]
    owner_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SeedFile {
    marketplaces: std::collections::HashMap<String, SeedMarketplace>,
    plugins: Vec<SeedPluginEntry>,
}

/// Lookup zh-CN description for `(plugin_name, skill_name)`.
/// Used to translate English SKILL.md frontmatter on installed skills.
fn lookup_zh_description(plugin_name: &str, skill_name: &str) -> Option<String> {
    let seed: SeedFile = serde_json::from_str(SEED_SKILLS_JSON).ok()?;
    seed.plugins
        .into_iter()
        .find(|p| p.name == plugin_name)?
        .bundled_skills?
        .into_iter()
        .find(|s| s.name == skill_name)?
        .description_zh
}

const SEED_SKILLS_JSON: &str = include_str!("seed/seed-skills.json");

/// Parse a SKILL.md YAML frontmatter for `name` and `description`.
/// Returns None if the file doesn't start with `---`.
fn parse_skill_md(path: &Path) -> Option<(String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = trimmed.strip_prefix("---")?.trim_start_matches('\n');
    let end_idx = after_first.find("\n---")?;
    let frontmatter = &after_first[..end_idx];

    let mut name = None;
    let mut description = None;
    for line in frontmatter.lines() {
        if let Some(rest) = line.strip_prefix("name:") {
            name = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(rest) = line.strip_prefix("description:") {
            description = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    Some((name?, description.unwrap_or_default()))
}

fn scan_skills_dir(dir: &Path, source: SkillSource, plugin_name: Option<String>) -> Vec<SkillMeta> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let Some((name, description)) = parse_skill_md(&skill_md) else {
            continue;
        };

        // Translate English SKILL.md description to zh-CN if we have it in the seed.
        let description = plugin_name
            .as_deref()
            .and_then(|p| lookup_zh_description(p, &name))
            .unwrap_or(description);

        let id = format!(
            "{}::{}",
            match source {
                SkillSource::User => "user",
                SkillSource::Project => "project",
                SkillSource::Plugin => "plugin",
                SkillSource::Recommend => "recommend",
            },
            name
        );
        out.push(SkillMeta {
            id,
            name,
            description,
            source: source.clone(),
            plugin_name: plugin_name.clone(),
            path: skill_md.to_string_lossy().to_string(),
            installed: true,
            category: None,
            bundled_skills: None,
            marketplace_id: None,
            marketplace_add_arg: None,
            marketplace_owner_label: None,
        });
    }
    out
}

/// Schema of `~/.claude/plugins/installed_plugins.json` (Claude Code's manifest).
#[derive(Debug, Deserialize)]
struct InstalledPluginsManifest {
    plugins: std::collections::HashMap<String, Vec<InstalledPluginEntry>>,
}

#[derive(Debug, Deserialize)]
struct InstalledPluginEntry {
    #[serde(rename = "installPath")]
    install_path: String,
}

/// Read `installed_plugins.json` and yield `(plugin_name_only, install_path)`.
/// `plugin_name_only` strips the `@<marketplace>` suffix.
fn read_installed_plugins() -> Vec<(String, std::path::PathBuf)> {
    let Some(home) = appfs::claude_home() else {
        return Vec::new();
    };
    let manifest_path = home.join("plugins").join("installed_plugins.json");
    let Ok(content) = std::fs::read_to_string(&manifest_path) else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<InstalledPluginsManifest>(&content) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (full_id, entries) in manifest.plugins {
        let plugin_name = full_id.split('@').next().unwrap_or(&full_id).to_string();
        if let Some(entry) = entries.into_iter().next() {
            out.push((plugin_name, std::path::PathBuf::from(entry.install_path)));
        }
    }
    out
}

pub fn list_installed_skills(workdir: Option<&Path>) -> Result<Vec<SkillMeta>> {
    let mut all = Vec::new();

    if let Some(home) = appfs::claude_home() {
        // User-level standalone skills (no plugin)
        let user_skills = home.join("skills");
        if user_skills.is_dir() {
            all.extend(scan_skills_dir(&user_skills, SkillSource::User, None));
        }

        // Plugin-bundled skills: read manifest for authoritative install paths.
        // Avoids guessing the plugin layout (which is actually
        // ~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/skills/).
        for (plugin_name, install_path) in read_installed_plugins() {
            let skills_dir = install_path.join("skills");
            if skills_dir.is_dir() {
                all.extend(scan_skills_dir(
                    &skills_dir,
                    SkillSource::Plugin,
                    Some(plugin_name),
                ));
            }
        }
    }

    if let Some(wd) = workdir {
        let project_skills = wd.join(".claude").join("skills");
        if project_skills.is_dir() {
            all.extend(scan_skills_dir(
                &project_skills,
                SkillSource::Project,
                None,
            ));
        }
    }

    Ok(all)
}

/// Returns recommended *plugins* from the bundled seed file.
/// Filters out plugins whose name matches an already-installed plugin
/// (looking at `plugin_name` of installed skills, since installed skills
/// were extracted from a plugin folder).
pub fn list_recommended_skills(installed: &[SkillMeta]) -> Vec<SkillMeta> {
    let Ok(seed) = serde_json::from_str::<SeedFile>(SEED_SKILLS_JSON) else {
        return Vec::new();
    };
    let installed_plugin_names: std::collections::HashSet<&str> = installed
        .iter()
        .filter_map(|s| s.plugin_name.as_deref())
        .collect();

    seed.plugins
        .into_iter()
        .filter(|p| !installed_plugin_names.contains(p.name.as_str()))
        .map(|p| {
            let mp = seed.marketplaces.get(&p.marketplace_id);
            SkillMeta {
                id: format!("recommend::{}::{}", p.marketplace_id, p.name),
                name: p.name,
                description: p.description,
                source: SkillSource::Recommend,
                plugin_name: None,
                path: String::new(),
                installed: false,
                category: p.category,
                bundled_skills: p.bundled_skills.map(|bs| {
                    bs.into_iter()
                        .map(|s| BundledSkillView {
                            name: s.name,
                            description_zh: s.description_zh.unwrap_or_default(),
                        })
                        .collect()
                }),
                marketplace_id: Some(p.marketplace_id.clone()),
                marketplace_add_arg: mp.map(|m| m.marketplace_add_arg.clone()),
                marketplace_owner_label: mp.and_then(|m| m.owner_label.clone()),
            }
        })
        .collect()
}

pub fn read_skill_md(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsReport {
    pub installed: Vec<SkillMeta>,
    pub recommended: Vec<SkillMeta>,
}

pub fn list_all(workdir: Option<&Path>) -> Result<SkillsReport> {
    let installed = list_installed_skills(workdir)?;
    let recommended = list_recommended_skills(&installed);
    Ok(SkillsReport {
        installed,
        recommended,
    })
}
