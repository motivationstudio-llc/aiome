use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::general::get_logs,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_setting,
        crate::routes::settings::test_connection,
        crate::routes::skill::list_skills,
        crate::routes::skill::import_skill,
        crate::routes::general::get_health_status
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
            aiome_core::contracts::SystemSetting
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
        std::fs::write(docs_dir.join("openapi.json"), schema).expect("Failed to write OpenAPI schema");
    }
}