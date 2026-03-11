use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::general::get_logs
    ),
    components(
        schemas(
            crate::routes::general::LogEntryResponse
        )
    ),
    info(
        title = "Aiome Management Console API",
        version = "0.1.0",
        description = "Core API for the Autonomous AI Operating System"
    )
)]
pub struct ApiDoc;

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