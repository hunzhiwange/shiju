use crate::model::AieosIdentity;

/// 将 AIEOS 身份转换为系统提示字符串。
///
/// 将 AIEOS 数据格式化为结构化的 markdown 提示文本，
/// 与 VibeWindow 的代理系统兼容。
pub fn aieos_to_system_prompt(identity: &AieosIdentity) -> String {
    use std::fmt::Write;

    let mut prompt = String::new();

    if let Some(ref id) = identity.identity {
        prompt.push_str("## Identity\n\n");
        prompt.push_str("## 身份信息\n\n");

        if let Some(ref names) = id.names {
            if let Some(ref first) = names.first {
                let _ = writeln!(prompt, "**Name:** {}", first);
                let _ = writeln!(prompt, "**姓名:** {}", first);
                if let Some(ref last) = names.last {
                    let _ = writeln!(prompt, "**Full Name:** {} {}", first, last);
                    let _ = writeln!(prompt, "**全名:** {} {}", first, last);
                }
            } else if let Some(ref full) = names.full {
                let _ = writeln!(prompt, "**Name:** {}", full);
                let _ = writeln!(prompt, "**姓名:** {}", full);
            }

            if let Some(ref nickname) = names.nickname {
                let _ = writeln!(prompt, "**Nickname:** {}", nickname);
                let _ = writeln!(prompt, "**昵称:** {}", nickname);
            }
        }

        if let Some(ref bio) = id.bio {
            let _ = writeln!(prompt, "**Bio:** {}", bio);
            let _ = writeln!(prompt, "**简介:** {}", bio);
        }

        if let Some(ref origin) = id.origin {
            let _ = writeln!(prompt, "**Origin:** {}", origin);
            let _ = writeln!(prompt, "**出生地:** {}", origin);
        }

        if let Some(ref residence) = id.residence {
            let _ = writeln!(prompt, "**居住地:** {}", residence);
        }

        prompt.push('\n');
    }

    if let Some(ref psych) = identity.psychology {
        prompt.push_str("## Personality\n\n");
        prompt.push_str("## 性格特征\n\n");

        if let Some(ref mbti) = psych.mbti {
            let _ = writeln!(prompt, "**MBTI:** {}", mbti);
        }

        if let Some(ref ocean) = psych.ocean {
            prompt.push_str("**OCEAN Traits:**\n");
            prompt.push_str("**OCEAN 人格特质:**\n");
            if let Some(openness) = ocean.openness {
                let _ = writeln!(prompt, "- Openness: {:.2}", openness);
                let _ = writeln!(prompt, "- 开放性: {:.2}", openness);
            }
            if let Some(conscientiousness) = ocean.conscientiousness {
                let _ = writeln!(prompt, "- Conscientiousness: {:.2}", conscientiousness);
                let _ = writeln!(prompt, "- 尽责性: {:.2}", conscientiousness);
            }
            if let Some(extraversion) = ocean.extraversion {
                let _ = writeln!(prompt, "- 外向性: {:.2}", extraversion);
            }
            if let Some(agreeableness) = ocean.agreeableness {
                let _ = writeln!(prompt, "- 宜人性: {:.2}", agreeableness);
            }
            if let Some(neuroticism) = ocean.neuroticism {
                let _ = writeln!(prompt, "- 神经质: {:.2}", neuroticism);
            }
        }

        if let Some(ref matrix) = psych.neural_matrix {
            if !matrix.is_empty() {
                prompt.push_str("\n**神经矩阵（认知权重）:**\n");
                let mut sorted_keys: Vec<_> = matrix.keys().collect();
                sorted_keys.sort();
                for trait_name in sorted_keys {
                    let weight = matrix.get(trait_name).unwrap();
                    let _ = writeln!(prompt, "- {}: {:.2}", trait_name, weight);
                }
            }
        }

        if let Some(ref compass) = psych.moral_compass {
            if !compass.is_empty() {
                prompt.push_str("\n**道德指南针:**\n");
                for principle in compass {
                    let _ = writeln!(prompt, "- {}", principle);
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(ref ling) = identity.linguistics {
        prompt.push_str("## Communication Style\n\n");
        prompt.push_str("## 沟通风格\n\n");

        if let Some(ref style) = ling.style {
            let _ = writeln!(prompt, "**Style:** {}", style);
            let _ = writeln!(prompt, "**风格:** {}", style);
        }

        if let Some(ref formality) = ling.formality {
            let _ = writeln!(prompt, "**Formality Level:** {}", formality);
            let _ = writeln!(prompt, "**正式程度:** {}", formality);
        }

        if let Some(ref phrases) = ling.catchphrases {
            if !phrases.is_empty() {
                prompt.push_str("**口头禅:**\n");
                for phrase in phrases {
                    let _ = writeln!(prompt, "- \"{}\"", phrase);
                }
            }
        }

        if let Some(ref forbidden) = ling.forbidden_words {
            if !forbidden.is_empty() {
                prompt.push_str("\n**Words/Phrases to Avoid:**\n");
                prompt.push_str("\n**避免使用的词语:**\n");
                for word in forbidden {
                    let _ = writeln!(prompt, "- {}", word);
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(ref mot) = identity.motivations {
        prompt.push_str("## Motivations\n\n");
        prompt.push_str("## 动机驱动\n\n");

        if let Some(ref drive) = mot.core_drive {
            let _ = writeln!(prompt, "**Core Drive:** {}", drive);
            let _ = writeln!(prompt, "**核心驱动力:** {}", drive);
        }

        if let Some(ref short) = mot.short_term_goals {
            if !short.is_empty() {
                prompt.push_str("**Short-term Goals:**\n");
                prompt.push_str("**短期目标:**\n");
                for goal in short {
                    let _ = writeln!(prompt, "- {}", goal);
                }
            }
        }

        if let Some(ref long) = mot.long_term_goals {
            if !long.is_empty() {
                prompt.push_str("\n**Long-term Goals:**\n");
                prompt.push_str("\n**长期目标:**\n");
                for goal in long {
                    let _ = writeln!(prompt, "- {}", goal);
                }
            }
        }

        if let Some(ref fears) = mot.fears {
            if !fears.is_empty() {
                prompt.push_str("\n**Fears/Avoidances:**\n");
                prompt.push_str("\n**恐惧/回避事项:**\n");
                for fear in fears {
                    let _ = writeln!(prompt, "- {}", fear);
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(ref cap) = identity.capabilities {
        prompt.push_str("## Capabilities\n\n");
        prompt.push_str("## 能力定义\n\n");

        if let Some(ref skills) = cap.skills {
            if !skills.is_empty() {
                prompt.push_str("**Skills:**\n");
                prompt.push_str("**技能:**\n");
                for skill in skills {
                    let _ = writeln!(prompt, "- {}", skill);
                }
            }
        }

        if let Some(ref tools) = cap.tools {
            if !tools.is_empty() {
                prompt.push_str("\n**Tools Access:**\n");
                prompt.push_str("\n**可访问工具:**\n");
                for tool in tools {
                    let _ = writeln!(prompt, "- {}", tool);
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(ref hist) = identity.history {
        prompt.push_str("## Background\n\n");
        prompt.push_str("## 背景故事\n\n");

        if let Some(ref story) = hist.origin_story {
            let _ = writeln!(prompt, "**Origin Story:** {}", story);
            let _ = writeln!(prompt, "**起源故事:** {}", story);
        }

        if let Some(ref education) = hist.education {
            if !education.is_empty() {
                prompt.push_str("**Education:**\n");
                prompt.push_str("**教育经历:**\n");
                for edu in education {
                    let _ = writeln!(prompt, "- {}", edu);
                }
            }
        }

        if let Some(ref occupation) = hist.occupation {
            let _ = writeln!(prompt, "\n**Occupation:** {}", occupation);
            let _ = writeln!(prompt, "\n**职业:** {}", occupation);
        }

        prompt.push('\n');
    }

    if let Some(ref phys) = identity.physicality {
        prompt.push_str("## Appearance\n\n");
        prompt.push_str("## 外貌特征\n\n");

        if let Some(ref appearance) = phys.appearance {
            let _ = writeln!(prompt, "{}", appearance);
        }

        if let Some(ref avatar) = phys.avatar_description {
            let _ = writeln!(prompt, "**Avatar Description:** {}", avatar);
            let _ = writeln!(prompt, "**头像描述:** {}", avatar);
        }

        prompt.push('\n');
    }

    if let Some(ref interests) = identity.interests {
        prompt.push_str("## Interests\n\n");
        prompt.push_str("## 兴趣爱好\n\n");

        if let Some(ref hobbies) = interests.hobbies {
            if !hobbies.is_empty() {
                prompt.push_str("**Hobbies:**\n");
                prompt.push_str("**爱好:**\n");
                for hobby in hobbies {
                    let _ = writeln!(prompt, "- {}", hobby);
                }
            }
        }

        if let Some(ref favorites) = interests.favorites {
            if !favorites.is_empty() {
                prompt.push_str("\n**Favorites:**\n");
                prompt.push_str("\n**喜好:**\n");
                let mut sorted_keys: Vec<_> = favorites.keys().collect();
                sorted_keys.sort();
                for category in sorted_keys {
                    let value = favorites.get(category).unwrap();
                    let _ = writeln!(prompt, "- {}: {}", category, value);
                }
            }
        }

        if let Some(ref lifestyle) = interests.lifestyle {
            let _ = writeln!(prompt, "\n**Lifestyle:** {}", lifestyle);
            let _ = writeln!(prompt, "\n**生活方式:** {}", lifestyle);
        }

        prompt.push('\n');
    }

    prompt.trim().to_string()
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
