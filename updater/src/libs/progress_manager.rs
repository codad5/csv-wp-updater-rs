use colored::*;
use redis::{AsyncCommands, Client, RedisResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;

static REDIS_CLIENT: OnceCell<Client> = OnceCell::const_new();

// Add these new structs to your processing_result.rs or progress_manager.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedRowDetail {
    pub row_number: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedProductDetail {
    pub sku: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStage {
    Starting,
    ParsingCsv,
    GroupingProducts,
    GroupingProductsCompleted {
        total_parents: usize,
        total_products: usize,
    },
    ProcessingProducts {
        sku: String,
        current: usize,
        total: usize,
    },
    ProcessingVariations {
        sku: String,
        current: usize,
        total: usize,
    },
    Finalizing,
    Completed,
    Failed(String),
}

impl ProcessingStage {
    pub fn to_message(&self) -> String {
        match self {
            ProcessingStage::Starting => "Initializing file processing...".to_string(),
            ProcessingStage::ParsingCsv => "Parsing CSV to extract product data...".to_string(),
            ProcessingStage::GroupingProducts => {
                "Organizing products and variations...".to_string()
            }
            ProcessingStage::GroupingProductsCompleted {
                total_parents,
                total_products,
            } => {
                format!(
                    "Found {} parent products with {} total items",
                    total_parents, total_products
                )
            }
            ProcessingStage::ProcessingProducts {
                sku,
                current,
                total,
            } => {
                format!("Processing product {} ({}/{})", sku, current, total)
            }
            ProcessingStage::ProcessingVariations {
                sku,
                current,
                total,
            } => {
                format!("Processing variation {} ({}/{})", sku, current, total)
            }
            ProcessingStage::Finalizing => "Finalizing and cleaning up...".to_string(),
            ProcessingStage::Completed => "Processing completed successfully".to_string(),
            ProcessingStage::Failed(error) => format!("Processing failed: {}", error),
        }
    }
}

// In progress_manager.rs - Update ProcessingProgress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingProgress {
    pub file_id: String,
    pub percent: f32,
    pub stage: ProcessingStage,
    pub stage_message: String, // NEW - Add this field
    pub total_rows: usize,
    pub processed_rows: usize,
    pub successful_rows: usize,
    pub failed_rows: usize,
    pub failed_row_details: Vec<FailedRowDetail>, // NEW
    pub failed_product_details: Vec<FailedProductDetail>, // NEW
    pub start_time: std::time::SystemTime,
    pub last_updated: std::time::SystemTime,
}

impl ProcessingProgress {
    pub fn new(file_id: String, total_rows: usize) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            file_id,
            percent: 0.0,
            stage: ProcessingStage::Starting,
            stage_message: ProcessingStage::Starting.to_message(), // NEW
            total_rows,
            processed_rows: 0,
            successful_rows: 0,
            failed_rows: 0,
            failed_row_details: Vec::new(),     // NEW
            failed_product_details: Vec::new(), // NEW
            start_time: now,
            last_updated: now,
        }
    }

    pub fn update_stage(&mut self, stage: ProcessingStage) {
        self.stage_message = stage.to_message();
        self.stage = stage;
        self.last_updated = std::time::SystemTime::now();

        // Auto-calculate percentage based on stage
        self.percent = match &self.stage {
            ProcessingStage::Starting => 0.0,
            ProcessingStage::ParsingCsv => 5.0,
            ProcessingStage::GroupingProducts => 10.0,
            ProcessingStage::GroupingProductsCompleted { .. } => 15.0,
            ProcessingStage::ProcessingProducts { current, total, .. } => {
                15.0 + ((*current as f32 / *total as f32) * 70.0)
            }
            ProcessingStage::ProcessingVariations { current, total, .. } => {
                15.0 + ((*current as f32 / *total as f32) * 70.0)
            }
            ProcessingStage::Finalizing => 90.0,
            ProcessingStage::Completed => 100.0,
            ProcessingStage::Failed(_) => self.percent, // Keep current percentage
        };
    }

    pub fn increment_processed(&mut self, success: bool) {
        self.processed_rows += 1;
        if success {
            self.successful_rows += 1;
        } else {
            self.failed_rows += 1;
        }
        self.last_updated = std::time::SystemTime::now();
    }
}

pub async fn get_redis_client() -> RedisResult<&'static Client> {
    REDIS_CLIENT
        .get_or_try_init(|| async {
            let redis_url =
                std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
            Client::open(redis_url)
        })
        .await
}

#[derive(Debug, Clone)]
pub struct ProgressManager {
    redis_client: &'static Client,
}

impl ProgressManager {
    pub async fn new() -> RedisResult<Self> {
        let redis_client = get_redis_client().await?;
        Ok(Self { redis_client })
    }

    pub async fn start_processing(
        &self,
        file_id: &str,
        total_rows: usize,
    ) -> RedisResult<ProcessingProgress> {
        let progress = ProcessingProgress::new(file_id.to_string(), total_rows);
        self.save_progress(&progress).await?;

        println!(
            "{}",
            format!("📈 Started processing file: {}", file_id).bright_green()
        );
        Ok(progress)
    }

    pub async fn update_stage(&self, file_id: &str, stage: ProcessingStage) -> RedisResult<()> {
        if let Ok(mut progress) = self.get_progress(file_id).await {
            progress.update_stage(stage.clone());
            self.save_progress(&progress).await?;

            println!(
                "{}",
                format!(
                    "🔄 [{}] {:.1}% - {}",
                    file_id,
                    progress.percent,
                    stage.to_message()
                )
                .bright_cyan()
            );
        }
        Ok(())
    }

    pub async fn increment_progress(&self, file_id: &str, success: bool) -> RedisResult<()> {
        if let Ok(mut progress) = self.get_progress(file_id).await {
            progress.increment_processed(success);
            self.save_progress(&progress).await?;
        }
        Ok(())
    }

    pub async fn complete_processing(&self, file_id: &str) -> RedisResult<()> {
        self.update_stage(file_id, ProcessingStage::Completed)
            .await?;

        if let Ok(progress) = self.get_progress(file_id).await {
            println!(
                "{}",
                format!(
                    "✅ Processing completed for {}: {}/{} successful, {}/{} failed",
                    file_id,
                    progress.successful_rows,
                    progress.total_rows,
                    progress.failed_rows,
                    progress.total_rows
                )
                .bright_green()
            );

            // Save final result to JSON file for history
            self.save_to_history(&progress).await.unwrap_or_else(|e| {
                println!("{}", format!("⚠️ Failed to save history: {}", e).yellow());
            });
        }
        Ok(())
    }

    pub async fn fail_processing(&self, file_id: &str, error: String) -> RedisResult<()> {
        self.update_stage(file_id, ProcessingStage::Failed(error.clone()))
            .await?;

        println!(
            "{}",
            format!("❌ Processing failed for {}: {}", file_id, error).bright_red()
        );
        Ok(())
    }

    pub async fn get_progress(&self, file_id: &str) -> RedisResult<ProcessingProgress> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let json: Option<String> = conn.get(format!("progress:{}", file_id)).await?;

        match json {
            Some(json_str) => serde_json::from_str(&json_str).map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::TypeError,
                    "JSON Parse Error",
                    e.to_string(),
                ))
            }),
            None => Err(redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Progress not found",
            ))),
        }
    }

    async fn save_progress(&self, progress: &ProcessingProgress) -> RedisResult<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let json = serde_json::to_string(progress).unwrap_or_default();

        // Store in Redis with TTL
        conn.set_ex(format!("progress:{}", progress.file_id), json, 3600)
            .await?;

        // Also store simple percentage for quick access
        conn.set_ex(
            format!("progress:percent:{}", progress.file_id),
            progress.percent,
            3600,
        )
        .await?;

        Ok(())
    }

    async fn save_to_history(
        &self,
        progress: &ProcessingProgress,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::helper::file_helper::get_report_path;

        let file_path = get_report_path(&progress.file_id);
        let json = serde_json::to_string_pretty(progress)?;
        tokio::fs::write(file_path, json).await?;

        Ok(())
    }

    // Quick methods for simple percentage retrieval
    pub async fn get_progress_percent(&self, file_id: &str) -> RedisResult<f32> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
        let percent: Option<f32> = conn.get(format!("progress:percent:{}", file_id)).await?;
        Ok(percent.unwrap_or(0.0))
    }

    pub async fn is_processing(&self, file_id: &str) -> RedisResult<bool> {
        match self.get_progress(file_id).await {
            Ok(progress) => Ok(!matches!(
                progress.stage,
                ProcessingStage::Completed | ProcessingStage::Failed(_)
            )),
            Err(_) => Ok(false),
        }
    }
}

impl ProgressManager {
    // NEW METHOD - Add failed row detail
    pub async fn add_failed_row(
        &self,
        file_id: &str,
        row_number: usize,
        reason: String,
    ) -> RedisResult<()> {
        if let Ok(mut progress) = self.get_progress(file_id).await {
            progress.failed_rows += 1;
            progress
                .failed_row_details
                .push(FailedRowDetail { row_number, reason });
            progress.last_updated = std::time::SystemTime::now();
            self.save_progress(&progress).await?;
        }
        Ok(())
    }

    // NEW METHOD - Add failed product detail
    pub async fn add_failed_product(
        &self,
        file_id: &str,
        sku: String,
        reason: String,
    ) -> RedisResult<()> {
        if let Ok(mut progress) = self.get_progress(file_id).await {
            progress
                .failed_product_details
                .push(FailedProductDetail { sku, reason });
            progress.last_updated = std::time::SystemTime::now();
            self.save_progress(&progress).await?;
        }
        Ok(())
    }
}
