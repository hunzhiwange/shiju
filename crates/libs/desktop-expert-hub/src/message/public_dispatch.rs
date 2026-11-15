use super::*;

pub(super) fn dispatch(gateway: crate::GatewayFactory, request: PublicRequest) -> Task<Message> {
    match request {
        PublicRequest::Sources { generation } => {
            Task::perform(gateway_skill_market_sources_async(gateway), move |result| {
                Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SourcesLoaded {
                    generation,
                    result,
                }))
            })
        }
        PublicRequest::Recommended { source, generation } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_recommended_async(gateway, &requested_source).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::RecommendedLoaded { source, generation, result },
                    ))
                },
            )
        }
        PublicRequest::Categories { source, generation } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_categories_async(gateway, &requested_source).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::CategoriesLoaded { source, generation, result },
                    ))
                },
            )
        }
        PublicRequest::Market { source, generation, query } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_skills_async(gateway, &requested_source, &query).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::MarketLoaded {
                        source,
                        generation,
                        result,
                    }))
                },
            )
        }
        PublicRequest::Suites { source, generation, query } => {
            let requested_source = source.clone();
            Task::perform(
                async move { gateway_skill_market_skill_sets_async(gateway, &requested_source, &query).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SuitesLoaded {
                        source,
                        generation,
                        result,
                    }))
                },
            )
        }
        PublicRequest::SkillDetail { locator, generation } => {
            let requested_locator = locator.clone();
            Task::perform(
                async move { gateway_skill_market_skill_detail_async(gateway, &requested_locator).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SkillDetailLoaded { locator, generation, result },
                    ))
                },
            )
        }
        PublicRequest::SkillFile { locator, generation } => {
            let requested_locator = locator.clone();
            Task::perform(
                async move { gateway_skill_market_skill_file_async(gateway, &requested_locator).await },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SkillFileLoaded { locator, generation, result },
                    ))
                },
            )
        }
        PublicRequest::SuiteDetail { source, slug, generation } => {
            let requested_source = source.clone();
            let requested_slug = slug.clone();
            Task::perform(
                async move {
                    gateway_skill_market_skill_set_detail_async(gateway, &requested_source, &requested_slug)
                        .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::SuiteDetailLoaded { source, slug, generation, result },
                    ))
                },
            )
        }
        PublicRequest::InstallSkill {
            project_path,
            coordinate,
            installations,
            target,
            generation,
        } => {
            let requested_project = project_path.clone();
            Task::perform(
                async move {
                    gateway_skill_market_install_to_targets_async(gateway,
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &coordinate,
                        &installations,
                    )
                    .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::InstallFinished {
                            target,
                            generation,
                            project_path,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::InstallSuite { project_path, source, slug, target, generation } => {
            let requested_project = project_path.clone();
            let requested_source = source.clone();
            let requested_slug = slug.clone();
            Task::perform(
                async move {
                    gateway_skill_market_skill_set_install_async(gateway,
                        &requested_project,
                        &requested_source,
                        &requested_slug,
                    )
                    .await
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::InstallFinished {
                            target,
                            generation,
                            project_path,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::RefreshInstalled { project_path, generation } => Task::perform(
            async move { gateway_skills_async(gateway, Some(&project_path)).await },
            move |result| {
                Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::InstalledRefreshed {
                    generation,
                    result,
                }))
            },
        ),
        PublicRequest::SetSkillEnabled { project_path, skill_id, enabled, generation } => {
            let requested_project = project_path.clone();
            let requested_skill = skill_id.clone();
            Task::perform(
                async move {
                    gateway_skill_set_enabled_async(gateway,
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &requested_skill,
                        enabled,
                    )
                    .await
                    .map(|_| ())
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::BatchItemFinished {
                            generation,
                            project_path,
                            skill_id,
                            result,
                        },
                    ))
                },
            )
        }
        PublicRequest::DeleteSkill { project_path, skill_id, generation } => {
            let requested_project = project_path.clone();
            let requested_skill = skill_id.clone();
            Task::perform(
                async move {
                    gateway_skill_delete_async(gateway,
                        (!requested_project.is_empty()).then_some(requested_project.as_str()),
                        &requested_skill,
                    )
                    .await
                    .map(|_| ())
                },
                move |result| {
                    Message::SkillHub(SkillHubMessage::Public(
                        PublicSkillMessage::BatchItemFinished {
                            generation,
                            project_path,
                            skill_id,
                            result,
                        },
                    ))
                },
            )
        }
    }
}
