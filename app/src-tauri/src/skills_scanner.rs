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
    pub repo_path: Option<String>,
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
struct SeedSkillEntry {
    name: String,
    description: String,
    category: Option<String>,
    #[serde(rename = "repoPath")]
    repo_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SeedFile {
    skills: Vec<SeedSkillEntry>,
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
            repo_path: None,
        });
    }
    out
}

pub fn list_installed_skills(workdir: Option<&Path>) -> Result<Vec<SkillMeta>> {
    let mut all = Vec::new();

    if let Some(home) = appfs::claude_home() {
        let user_skills = home.join("skills");
        if user_skills.is_dir() {
            all.extend(scan_skills_dir(&user_skills, SkillSource::User, None));
        }

        let plugins_dir = home.join("plugins");
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let plugin_path = entry.path();
                if !plugin_path.is_dir() {
                    continue;
                }
                let plugin_name = plugin_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(String::from);
                let inner = plugin_path.join("skills");
                if inner.is_dir() {
                    all.extend(scan_skills_dir(&inner, SkillSource::Plugin, plugin_name));
                }
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

pub fn list_recommended_skills(installed: &[SkillMeta]) -> Vec<SkillMeta> {
    let Ok(seed) = serde_json::from_str::<SeedFile>(SEED_SKILLS_JSON) else {
        return Vec::new();
    };
    let installed_names: std::collections::HashSet<&str> =
        installed.iter().map(|s| s.name.as_str()).collect();

    seed.skills
        .into_iter()
        .filter(|s| !installed_names.contains(s.name.as_str()))
        .map(|s| SkillMeta {
            id: format!("recommend::{}", s.name),
            name: s.name.clone(),
            description: s.description,
            source: SkillSource::Recommend,
            plugin_name: None,
            path: s.repo_path.clone().unwrap_or_default(),
            installed: false,
            category: s.category,
            repo_path: s.repo_path,
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
