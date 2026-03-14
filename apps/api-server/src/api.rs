/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::general::get_logs,
        crate::routes::general::list_wiki_files,
        crate::routes::general::get_wiki_content,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_setting,
        crate::routes::settings::test_connection,
        crate::routes::settings::get_ollama_models,
        crate::routes::skill::list_skills,
        crate::routes::skill::import_skill,
        crate::routes::skill::spawn_mcp_server,
        crate::routes::general::get_health_status,
        // Agent
        crate::routes::agent::trigger_agent_chat,
        crate::routes::agent::handle_karma_feedback,
        // Karma
        crate::routes::karma::get_karma_stream,
        crate::routes::karma::trigger_failure_demo,
        crate::routes::karma::trigger_security_demo,
        crate::routes::karma::trigger_federation_demo,
        crate::routes::karma::synergy_graph_handler,
        crate::routes::karma::get_immune_rules_handler,
        crate::routes::karma::add_immune_rule_handler,
        crate::routes::karma::delete_immune_rule_handler,
        crate::routes::karma::get_evolution_history_handler,
        // Biome
        crate::routes::biome::biome_status,
        crate::routes::biome::list_topics,
        crate::routes::biome::create_topic,
        crate::routes::biome::autonomous_start,
        crate::routes::biome::autonomous_stop,
        crate::routes::biome::autonomous_status,
        crate::routes::biome::list_messages,
        crate::routes::biome::send_message,
        // Expression
        crate::routes::expression::expression_status,
        crate::routes::expression::generate_expression,
        crate::routes::expression::list_expressions,
        crate::routes::expression::toggle_auto_expression,
        // Artifacts
        crate::routes::artifacts::list_artifacts_handler,
        crate::routes::artifacts::get_artifact_handler,
        crate::routes::artifacts::download_artifact_file_handler,
        crate::routes::artifacts::delete_artifact_handler,
        crate::routes::artifacts::get_artifact_edges_handler
    ),
    components(
        schemas(
            crate::routes::general::LogEntryResponse,
            crate::routes::settings::UpdateSettingsRequest,
            crate::routes::settings::TestConnectionRequest,
            crate::routes::settings::TestConnectionResponse,
            crate::routes::skill::SkillSummary,
            crate::routes::skill::ImportRequest,
            crate::routes::skill::McpSpawnRequest,
            shared::health::ResourceStatus,
            aiome_core::contracts::SystemSetting,
            aiome_core::contracts::ImmuneRule,
            crate::routes::agent::AgentChatRequest,
            crate::routes::agent::ChatMessage,
            crate::routes::agent::KarmaFeedbackRequest,
            crate::routes::karma::GraphNode,
            crate::routes::karma::GraphEdge,
            crate::routes::karma::GraphData,
            crate::routes::biome::SendBiomeRequest,
            crate::routes::biome::StartAutonomousRequest,
            crate::routes::expression::ListParams,
            crate::routes::expression::AutoToggle,
            crate::routes::artifacts::ListArtifactsParams
        )
    ),
    info(
        title = "Aiome Management Console API",
        version = "0.1.0",
        description = "Core API for the Autonomous AI Operating System"
    ),
    modifiers(&SecurityAddon)
)
]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("API_SERVER_SECRET")
                        .build(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_schema_generation() {
        let schema = ApiDoc::openapi().to_pretty_json().unwrap();
        assert!(!schema.is_empty());

        let docs_dir = std::path::Path::new("../../docs");
        if !docs_dir.exists() {
            std::fs::create_dir_all(docs_dir).unwrap();
        }
        std::fs::write(docs_dir.join("openapi.json"), schema)
            .expect("Failed to write OpenAPI schema");
    }
}
