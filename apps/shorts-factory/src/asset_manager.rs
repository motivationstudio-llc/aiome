use std::path::PathBuf;
use factory_core::contracts::ConceptResponse;
use factory_core::error::FactoryError;
use tuning::StyleProfile;
use serde::{Serialize, Deserialize};

/// 中間素材と最終成果物の管理、および永続化 (Remix Mode の基盤)
pub struct AssetManager {
    base_dir: PathBuf,
}

impl AssetManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// プロジェクトディレクトリを初期化
    pub fn init_project(&self, project_id: &str) -> Result<PathBuf, FactoryError> {
        let path = self.base_dir.join(project_id);
        std::fs::create_dir_all(&path).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to create project dir: {}", e),
        })?;
        
        // サブディレクトリ作成
        std::fs::create_dir_all(path.join("visuals")).ok();
        std::fs::create_dir_all(path.join("audio")).ok();
        
        Ok(path)
    }

    /// コンセプトを保存
    pub fn save_concept(&self, project_id: &str, concept: &ConceptResponse) -> Result<(), FactoryError> {
        let path = self.base_dir.join(project_id).join("concept.json");
        let json = serde_json::to_string_pretty(concept).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to serialize concept: {}", e),
        })?;
        std::fs::write(path, json).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to write concept.json: {}", e),
        })
    }

    /// コンセプトを読み込み (自動マイグレーション対応)
    pub fn load_concept(&self, project_id: &str) -> Result<ConceptResponse, FactoryError> {
        let path = self.base_dir.join(project_id).join("concept.json");
        let content = std::fs::read_to_string(path).map_err(|e| FactoryError::MediaNotFound {
            path: format!("concept.json for {}: {}", project_id, e),
        })?;
        
        let mut concept: ConceptResponse = serde_json::from_str(&content).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to parse concept.json: {}", e),
        })?;

        // --- Backward Compatibility Migration ---
        // もし scripts が空で、旧形式の日本語台本が存在する場合、ja ロケールとして統合する
        if concept.scripts.is_empty() && !concept.script_intro.is_empty() {
             concept.scripts.push(factory_core::contracts::LocalizedScript {
                 lang: "ja".to_string(),
                 display_intro: concept.display_intro.clone(),
                 display_body: concept.display_body.clone(),
                 display_outro: concept.display_outro.clone(),
                 script_intro: concept.script_intro.clone(),
                 script_body: concept.script_body.clone(),
                 script_outro: concept.script_outro.clone(),
                 style_intro: concept.style_intro.clone(),
                 style_body: concept.style_body.clone(),
                 style_outro: concept.style_outro.clone(),
             });
        }

        Ok(concept)
    }

    /// 素材（動画・音声）の存在チェック
    #[allow(dead_code)]
    pub fn check_assets(&self, project_id: &str, scene_count: usize) -> bool {
        let root = self.base_dir.join(project_id);
        
        // 音声チェック
        for i in 0..scene_count {
            if !root.join(format!("audio/scene_{}.wav", i)).exists() {
                return false;
            }
        }
        
        // 動画チェック
        for i in 0..scene_count {
            if !root.join(format!("visuals/scene_{}.mp4", i)).exists() {
                return false;
            }
        }
        
        true
    }

    /// 最終的な実行パラメータをスナップショットとして保存
    pub fn save_metadata(&self, project_id: &str, style: &StyleProfile) -> Result<(), FactoryError> {
        let path = self.base_dir.join(project_id).join("metadata.json");
        let metadata = serde_json::json!({
            "project_id": project_id,
            "style_used": style,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        let json = serde_json::to_string_pretty(&metadata).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to serialize metadata: {}", e),
        })?;
        
        std::fs::write(path, json).map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to write metadata.json: {}", e),
        })
    }

    /// ワークスペース内の全プロジェクトをスキャンして一覧を返す
    pub fn list_projects(&self) -> Vec<ProjectSummary> {
        let mut projects = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let project_id = entry.file_name().to_string_lossy().to_string();
                        // 隠しディレクトリ等はスキップ
                        if project_id.starts_with('.') { continue; }

                        if let Some(summary) = self.read_project_summary(&project_id) {
                            projects.push(summary);
                        }
                    }
                }
            }
        }
        
        // 新しい順にソート
        projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        projects
    }

    fn read_project_summary(&self, project_id: &str) -> Option<ProjectSummary> {
        let root = self.base_dir.join(project_id);
        
        // Metadata (Timestamp, Style)
        let meta_path = root.join("metadata.json");
        let (timestamp, style) = if meta_path.exists() {
            let content = std::fs::read_to_string(&meta_path).ok()?;
            let json: serde_json::Value = serde_json::from_str(&content).ok()?;
            (
                json["timestamp"].as_str().unwrap_or("").to_string(),
                json["style_used"]["name"].as_str().map(|s| s.to_string())
            )
        } else {
            // metadataがない場合はディレクトリの更新日時等を代用すべきだが、今回はスキップ
            // または concept.json だけあれば表示する方針もアリ
            return None;
        };

        // Concept (Title)
        let concept_path = root.join("concept.json");
        let title = if concept_path.exists() {
            let content = std::fs::read_to_string(&concept_path).ok()?;
            let json: serde_json::Value = serde_json::from_str(&content).ok()?;
            json["title"].as_str().unwrap_or(project_id).to_string()
        } else {
            project_id.to_string()
        };

        // Thumbnail (Priority: thumb.png > final_video.mp4 (handled by frontend) > default)
        // ここではAPIとしてアクセス可能なパス ("/assets/...") を返す
        let thumb_path = if root.join("thumb.png").exists() {
            Some(format!("/assets/{}/thumb.png", project_id))
        } else if root.join("final.mp4").exists() {
            // フロントエンドで video タグの poster として使うか、動画そのものをサムネイル代わりにする
             Some(format!("/assets/{}/final.mp4", project_id))
        } else {
            None
        };

        Some(ProjectSummary {
            id: project_id.to_string(),
            title,
            style,
            created_at: timestamp,
            thumbnail_url: thumb_path,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub style: Option<String>,
    pub created_at: String,
    pub thumbnail_url: Option<String>,
}
