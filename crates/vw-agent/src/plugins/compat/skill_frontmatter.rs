//! 插件 Markdown 技能的严格元数据边界，不改变原始指令字节。

use anyhow::{Result, bail, ensure};
use serde::Deserialize;

#[derive(Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    metadata: Metadata,
}
#[derive(Default, Deserialize)]
struct Metadata {
    #[serde(rename = "short-description")]
    short_description: Option<String>,
}
pub(crate) struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
}

pub(crate) fn parse(content: &str, default_name: &str) -> Result<SkillFrontmatter> {
    let mut lines = content.lines();
    ensure!(
        lines.next().is_some_and(|line| line.trim() == "---"),
        "missing YAML frontmatter delimited by ---"
    );
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    ensure!(closed && !yaml.trim().is_empty(), "missing YAML frontmatter delimited by ---");
    let value: Frontmatter = serde_yaml::from_str(&yaml)
        .or_else(|original| serde_yaml::from_str(&repair_scalar_lines(&yaml)).map_err(|_| original))
        .map_err(|error| match error.location() {
            Some(location) => anyhow::anyhow!(
                "invalid skill YAML frontmatter at line {}, column {}",
                location.line(),
                location.column()
            ),
            None => anyhow::anyhow!("invalid skill YAML frontmatter"),
        })?;
    let name = value
        .name
        .as_deref()
        .map(single_line)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| default_name.to_owned());
    ensure!(
        !name.is_empty() && name.chars().count() <= 64,
        "skill name must contain 1 to 64 characters"
    );
    let description = value.description.as_deref().map(single_line).unwrap_or_default();
    if description.is_empty() {
        bail!("skill frontmatter is missing nonempty description");
    }
    Ok(SkillFrontmatter {
        name,
        description,
        short_description: value
            .metadata
            .short_description
            .as_deref()
            .map(single_line)
            .filter(|value| !value.is_empty()),
    })
}
fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Repair ambiguous unquoted prose only. Preserve block bodies and valid collections;
// reparsing must still reject unrelated YAML errors.
fn repair_scalar_lines(yaml: &str) -> String {
    let mut block_indent = None;
    yaml.lines()
        .map(|line| {
            let indent = line.chars().take_while(|ch| *ch == ' ').count();
            if let Some(block) = block_indent {
                if line.trim().is_empty() || indent > block {
                    return line.to_owned();
                }
                block_indent = None;
            }
            let Some((key, raw)) = line.split_once(':') else { return line.to_owned() };
            if key.trim().is_empty() || raw.chars().next().is_some_and(|ch| !ch.is_whitespace()) {
                return line.to_owned();
            }
            let value = raw.trim_start();
            let comment = value
                .char_indices()
                .find(|(index, ch)| {
                    *ch == '#'
                        && (*index == 0
                            || value[..*index].chars().next_back().is_some_and(char::is_whitespace))
                })
                .map(|(index, _)| value[..index].trim_end().len())
                .unwrap_or(value.len());
            let scalar = value[..comment].trim_end();
            let Some(first) = scalar.chars().next() else { return line.to_owned() };
            if matches!(first, '|' | '>') {
                block_indent = Some(indent);
                return line.to_owned();
            }
            if matches!(first, '\'' | '"') {
                return line.to_owned();
            }
            let ambiguous_colon = scalar.char_indices().any(|(index, ch)| {
                ch == ':' && scalar[index + 1..].chars().next().is_some_and(char::is_whitespace)
            });
            let malformed_flow = matches!(first, '[' | '{' | '@' | '`')
                && serde_yaml::from_str::<serde_yaml::Value>(scalar).is_err();
            if ambiguous_colon || malformed_flow {
                format!(
                    "{key}: {}{}",
                    serde_json::to_string(scalar).expect("string serialization"),
                    &value[comment..]
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
