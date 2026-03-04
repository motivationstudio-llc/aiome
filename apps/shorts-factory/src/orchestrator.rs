use factory_core::contracts::{
    ConceptRequest, TrendRequest, TrendResponse,
    VideoRequest, MediaRequest, MediaResponse,
    VoiceRequest, WorkflowRequest, WorkflowResponse
};
use factory_core::traits::{AgentAct, MediaEditor};
use factory_core::error::FactoryError;
use infrastructure::trend_sonar::BraveTrendSonar;
use infrastructure::concept_manager::ConceptManager;
use infrastructure::comfy_bridge::ComfyBridgeClient;
use infrastructure::media_forge::MediaForgeClient;
use infrastructure::voice_actor::VoiceActor;
use infrastructure::sound_mixer::SoundMixer;
use crate::supervisor::Supervisor;
use crate::arbiter::{ResourceArbiter, ResourceUser};
use crate::asset_manager::AssetManager;
use tuning::StyleManager;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// 映像量産統括者 (ProductionOrchestrator)
/// 
/// 複数のアクターを協調させ、トレンド分析から動画完成までのパイプラインを管理する。
pub struct ProductionOrchestrator {
    pub trend_sonar: BraveTrendSonar,
    pub concept_manager: ConceptManager,
    pub voice_actor: Arc<VoiceActor>,
    pub comfy_bridge: ComfyBridgeClient,
    pub media_forge: MediaForgeClient,
    pub sound_mixer: SoundMixer,
    pub supervisor: Supervisor,
    pub arbiter: Arc<ResourceArbiter>,
    pub style_manager: Arc<StyleManager>,
    pub asset_manager: Arc<AssetManager>,
    pub export_dir: String,
}

impl ProductionOrchestrator {
    pub fn new(
        trend_sonar: BraveTrendSonar,
        concept_manager: ConceptManager,
        voice_actor: Arc<VoiceActor>,
        comfy_bridge: ComfyBridgeClient,
        media_forge: MediaForgeClient,
        sound_mixer: SoundMixer,
        supervisor: Supervisor,
        arbiter: Arc<ResourceArbiter>,
        style_manager: Arc<StyleManager>,
        asset_manager: Arc<AssetManager>,
        export_dir: String,
    ) -> Self {
        Self {
            trend_sonar,
            concept_manager,
            voice_actor,
            comfy_bridge,
            media_forge,
            sound_mixer,
            supervisor,
            arbiter,
            style_manager,
            asset_manager,
            export_dir,
        }
    }
}

#[async_trait]
impl AgentAct for ProductionOrchestrator {
    type Input = WorkflowRequest;
    type Output = WorkflowResponse;

    async fn execute(
        &self,
        input: WorkflowRequest,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<WorkflowResponse, FactoryError> {
        info!("🏭 Aiome Video Forge: Starting Pipeline for topic '{}'", input.topic);

        // --- Phase 1: Concept & Setup ---
        let project_id = input.remix_id.unwrap_or_else(|| {
            format!("{}_{}", input.category, chrono::Utc::now().format("%Y%m%d_%H%M%S"))
        });
        let project_root = self.asset_manager.init_project(&project_id)?;
        
        // target_langs の決定（指定なしなら ja + en）
        let target_langs = if input.target_langs.is_empty() {
            vec!["ja".to_string(), "en".to_string()]
        } else {
            input.target_langs.clone()
        };

        // コンセプト取得
        let concept_res = if input.skip_to_step.is_some() {
             self.asset_manager.load_concept(&project_id)?
        } else {
            let trend_req = TrendRequest { category: input.category.clone() };
            let trend_res: TrendResponse = self.supervisor.enforce_act(&self.trend_sonar, trend_req).await?;
            let concept_req = ConceptRequest { 
                topic: input.topic.clone(),
                category: input.category.clone(),
                trend_items: trend_res.items,
                available_styles: self.style_manager.list_available_styles(),
                relevant_karma: input.relevant_karma.clone(),
                previous_attempt_log: input.previous_attempt_log.clone(),
            };
            let res = self.supervisor.enforce_act(&self.concept_manager, concept_req).await?;
            self.asset_manager.save_concept(&project_id, &res)?;
            res
        };

        // --- Phase 1.2: Honorable Abort (Pre-flight Strategic Scan) ---
        if concept_res.title.trim().is_empty() || concept_res.display_intro.chars().count() < 10 {
            return Err(FactoryError::HonorableAbort { 
                reason: "Concept density too low for production. Aborting to save resources.".into() 
            });
        }

        // Logic check for non-sensical titles or placeholder contamination
        if concept_res.title.to_lowercase().contains("placeholder") || concept_res.title.to_lowercase().contains("tbd") {
            return Err(FactoryError::HonorableAbort { 
                reason: "Placeholder content detected in generated concept.".into() 
            });
        }

        // スタイル決定
        let base_style_name = if !input.style_name.is_empty() { &input.style_name } else { &concept_res.style_profile };
        let mut style = self.style_manager.get_style(base_style_name);
        if let Some(custom) = &input.custom_style {
            if let Some(v) = custom.zoom_speed { style.zoom_speed = v; }
            if let Some(v) = custom.pan_intensity { style.pan_intensity = v; }
            if let Some(v) = custom.bgm_volume { style.bgm_volume = v; }
            if let Some(v) = custom.ducking_threshold { style.ducking_threshold = v; }
            if let Some(v) = custom.ducking_ratio { style.ducking_ratio = v; }
            if let Some(v) = custom.fade_duration { style.fade_duration = v; }
        }

        // 保存: スナップショット (Phase 1.5)
        self.asset_manager.save_metadata(&project_id, &style)?;

        // --- Phase 2: Asset Generation (Exclusive GPU Access) ---
        info!("💎 Phase 2: Asset Generation (GPU Exclusive)...");
        let mut audio_assets = std::collections::HashMap::new(); // lang -> Vec<PathBuf>
        let mut image_assets = Vec::new(); // Vec<PathBuf>

        {
            let _gpu_guard = self.arbiter.acquire_gpu(ResourceUser::Generating).await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Arbiter error: {}", e) })?;

            // 2.1. 画像生成 x 3 (Intro, Body, Outro)
            for (i, visual_prompt) in concept_res.visual_prompts.iter().enumerate() {
                let img_path = project_root.join(format!("visuals/scene_{}.png", i));
                if !img_path.exists() {
                    let full_prompt = format!("{}, {}", concept_res.common_style, visual_prompt);
                    let video_req = VideoRequest {
                        prompt: full_prompt,
                        workflow_id: "shorts_standard_v1".to_string(),
                        input_image: None,
                    };
                    let res = self.supervisor.enforce_act(&self.comfy_bridge, video_req).await?;
                    let temp_path = self.supervisor.jail().root().join(&res.output_path);
                    std::fs::create_dir_all(img_path.parent().unwrap()).ok();
                    std::fs::copy(&temp_path, &img_path).map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;
                    self.comfy_bridge.delete_output_debris(&res.job_id);
                }
                image_assets.push(img_path);
            }

            // 2.2. TTS生成 for each lang
            for lang in &target_langs {
                if let Some(script) = concept_res.scripts.iter().find(|s| &s.lang == lang) {
                    info!("🗣️ Generating TTS for language: {}", lang);
                    let mut lang_audios = Vec::new();
                    let sections = vec![
                        (&script.script_intro, &script.style_intro),
                        (&script.script_body, &script.style_body),
                        (&script.script_outro, &script.style_outro)
                    ];
                    
                    for (i, (script_text, style_name)) in sections.into_iter().enumerate() {
                        let audio_path = project_root.join(format!("audio/scene_{}_{}.wav", i, lang));
                        if !audio_path.exists() {
                            let voice_req = VoiceRequest {
                                text: script_text.clone(),
                                voice: String::new(), 
                                speed: None,
                                lang: Some(lang.clone()),
                                style: if style_name.is_empty() { None } else { Some(style_name.clone()) },
                                model_name: None, // Default in VoiceActor
                            };
                            let v_res = self.supervisor.enforce_act(&*self.voice_actor, voice_req).await?;
                            let temp_v = self.supervisor.jail().root().join(&v_res.audio_path);
                            std::fs::create_dir_all(audio_path.parent().unwrap()).ok();
                            std::fs::copy(&temp_v, &audio_path).map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;
                        }
                        lang_audios.push(audio_path);
                    }
                    audio_assets.insert(lang.clone(), lang_audios);
                }
            }
        } // GPU Guard released

        // --- Phase 3: Forge & Parallel Composition ---
        info!("🔥 Phase 3: Forge (Video Composition)...");
        let mut output_videos = Vec::new();

        for lang in &target_langs {
            if let (Some(audios), Some(script)) = (audio_assets.get(lang), concept_res.scripts.iter().find(|s| &s.lang == lang)) {
                let _forge_guard = self.arbiter.acquire_forge(ResourceUser::Forging).await
                    .map_err(|e| FactoryError::Infrastructure { reason: format!("Arbiter error: {}", e) })?;

                info!("🎬 Forging video for language: {}", lang);
                let lang_proj_root = project_root.join(lang);
                std::fs::create_dir_all(&lang_proj_root).ok();

                // 3.1. Ken Burns / Subtitle Generation
                let mut video_clips = Vec::new();
                let mut srt_content = String::new();
                let mut current_time = 0.0f32;
                let mut srt_index = 1;

                let displays = vec![&script.display_intro, &script.display_body, &script.display_outro];

                for (i, (img_path, audio_path)) in image_assets.iter().zip(audios.iter()).enumerate() {
                    let duration = self.media_forge.get_duration(audio_path).await.unwrap_or(5.0);
                    let clip_path = lang_proj_root.join(format!("clip_{}.mp4", i));
                    
                    // Ken Burns
                    let clip = self.comfy_bridge.apply_ken_burns_effect(img_path, duration, jail, &style).await?;
                    let temp_clip = self.supervisor.jail().root().join(clip);
                    std::fs::copy(&temp_clip, &clip_path).ok();
                    video_clips.push(clip_path);

                    // Subtitles
                    let sentences = split_into_sentences(displays[i]);
                    let total_chars: usize = sentences.iter().map(|s| s.chars().count()).sum();
                    let mut accumulated = 0.0f32;
                    for sentence in sentences {
                        let ratio = sentence.chars().count() as f32 / total_chars as f32;
                        let s_duration = duration * ratio;
                        let start = format_srt_time(current_time + accumulated);
                        let end = format_srt_time(current_time + accumulated + s_duration);
                        srt_content.push_str(&format!("{}\n{} --> {}\n{}\n\n", srt_index, start, end, sentence));
                        srt_index += 1;
                        accumulated += s_duration;
                    }
                    current_time += duration;
                }

                let srt_path = lang_proj_root.join("subtitles.srt");
                std::fs::write(&srt_path, srt_content).ok();

                // 3.2. Final Assembly per language
                let combined_v = self.media_forge.concatenate_clips(video_clips.iter().map(|p| p.to_string_lossy().to_string()).collect(), format!("v_{}.mp4", lang)).await?;
                let combined_a = self.media_forge.concatenate_clips(audios.iter().map(|p| p.to_string_lossy().to_string()).collect(), format!("a_{}.wav", lang)).await?;
                
                let finalized_a = lang_proj_root.join("final_audio.wav");
                self.sound_mixer.mix_and_finalize(&std::path::PathBuf::from(combined_a), &input.category, &finalized_a, &style).await?;

                let style_with_font = format!("Fontname={},FontSize={}", font_for_lang(lang), font_size_for_lang(lang));
                let media_req = MediaRequest {
                    video_path: combined_v,
                    audio_path: finalized_a.to_string_lossy().to_string(),
                    subtitle_path: Some(srt_path.to_string_lossy().to_string()),
                    force_style: Some(style_with_font),
                };
                
                let media_res: MediaResponse = self.supervisor.enforce_act(&self.media_forge, media_req).await?;

                let final_path = std::path::PathBuf::from(media_res.final_path);
                let delivered = infrastructure::workspace_manager::WorkspaceManager::deliver_output(
                    &format!("{}_{}", project_id, lang),
                    &final_path,
                    &self.export_dir,
                ).await?;

                output_videos.push(factory_core::contracts::OutputVideo {
                    lang: lang.clone(),
                    path: delivered.to_string_lossy().to_string(),
                });
            }
        }

        let first_path = output_videos.first().map(|v| v.path.clone()).unwrap_or_default();
        
        info!("🏆 Aiome Video Forge: Pipeline Completed for {} languages", output_videos.len());

        Ok(WorkflowResponse {
            final_video_path: first_path,
            output_videos,
            concept: concept_res,
        })
    }
}

/// 言語別フォントマッピング
fn font_for_lang(lang: &str) -> &str {
    match lang {
        "ja" => "Noto Sans JP Black",
        "en" => "Inter Bold",
        _ => "Noto Sans Bold",
    }
}

/// 言語別デフォルトフォントサイズ
fn font_size_for_lang(lang: &str) -> i32 {
    match lang {
        "ja" => 18,
        "en" => 12, // 英語は単語数が多くなりやすいため大幅に縮小
        _ => 16,
    }
}

/// SRT 形式のタイムスタンプ文字列を生成 (HH:MM:SS,mmm)
fn format_srt_time(secs: f32) -> String {
    let hours = (secs / 3600.0) as u32;
    let minutes = ((secs % 3600.0) / 60.0) as u32;
    let seconds = (secs % 60.0) as u32;
    let millis = ((secs % 1.0) * 1000.0) as u32;
    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
}

/// テキストを句読点や改行で文章単位に分割する。
/// 英語の場合はピリオド等でも分割し、かつ長すぎる場合はスペースでチャンク分けする。
fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    
    // 英語と日本語の両方の句切りに対応
    let delimiters = ['。', '？', '！', '.', '?', '!', '\n'];
    
    for c in text.chars() {
        current.push(c);
        
        let should_split = if delimiters.contains(&c) {
            true
        } else if (c == ' ' || c == '、' || c == ',') && current.chars().count() > 30 {
            // 30文字を超えていて、区切り（スペース、読点、コンマ）があれば分割
            true
        } else {
            false
        };

        if should_split {
            let s = current.trim().to_string();
            if !s.is_empty() {
                sentences.push(s);
            }
            current.clear();
        }
    }
    
    // 残りのテキスト
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    
    sentences
}
