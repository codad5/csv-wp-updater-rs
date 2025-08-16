use crate::libs::progress_manager::{ProcessingStage, ProgressManager};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedRow {
    pub row_number: usize,
    pub reason: String,
    pub sku: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductProcessType {
    Parent,
    Child,
    Standalone,
}

#[derive(Debug, Clone)]
pub struct ProcessedRowInfo {
    pub row_number: usize,
    pub processing_time: Duration,
    pub processed_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ProcessedProductInfo {
    pub sku: String,
    pub product_type: ProductProcessType,
    pub row_number: usize,
    pub processing_time: Duration,
    pub processed_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingAnalytics {
    pub total_rows_processed: usize,
    pub total_products_processed: usize,
    pub average_row_time: Duration,
    pub parent_products: usize,
    pub child_products: usize,
    pub failed_rows: usize,
    pub success_rate: f64,
    pub processing_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ProcessingResult {
    pub file_name: String,
    pub status: ProcessingStatus,
    processed_rows: HashSet<usize>,
    row_details: HashMap<usize, ProcessedRowInfo>,
    product_details: HashMap<String, ProcessedProductInfo>,
    pub total_row: usize,
    pub failed_row: Vec<FailedRow>,
    pub start_time: Instant,
    pub processing_time: Duration,
    pub total_products: usize,
    pub parent_products: usize,
    pub child_products: usize,

    // Progress management integration
    progress_manager: ProgressManager,
    pub file_id: String,
}

impl ProcessingResult {
    pub async fn new(
        file_name: String,
        file_id: String,
        total_rows: usize,
    ) -> Result<Self, redis::RedisError> {
        let start_time = Instant::now();
        let progress_manager = ProgressManager::new().await?;

        // Initialize progress tracking
        progress_manager
            .start_processing(&file_id, total_rows)
            .await?;

        Ok(ProcessingResult {
            file_name,
            file_id,
            status: ProcessingStatus::Success,
            total_row: total_rows,
            processed_rows: HashSet::new(),
            row_details: HashMap::new(),
            product_details: HashMap::new(),
            failed_row: Vec::new(),
            start_time,
            processing_time: Duration::default(),
            total_products: 0,
            parent_products: 0,
            child_products: 0,
            progress_manager,
        })
    }

    pub async fn update_stage(&self, stage: ProcessingStage) -> Result<(), redis::RedisError> {
        self.progress_manager
            .update_stage(&self.file_id, stage)
            .await
    }

    pub async fn set_csv_parsing_complete(&self) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::ParsingCsv).await
    }

    pub async fn set_grouping_products(&self) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::GroupingProducts).await
    }

    pub async fn set_grouping_complete(
        &self,
        total_parents: usize,
        total_products: usize,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::GroupingProductsCompleted {
            total_parents,
            total_products,
        })
        .await
    }

    pub async fn start_processing_product(
        &self,
        sku: String,
        current: usize,
        total: usize,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::ProcessingProducts {
            sku,
            current,
            total,
        })
        .await
    }

    pub async fn start_processing_variation(
        &self,
        sku: String,
        parent_sku: String,
        current: usize,
        total: usize,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::ProcessingVariations {
            sku,
            parent_sku,
            current,
            total,
        })
        .await
    }

    pub async fn finalizing(&self) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::Finalizing).await
    }

    pub async fn complete(&mut self) -> Result<(), redis::RedisError> {
        self.processing_time = self.start_time.elapsed();
        self.progress_manager
            .complete_processing(&self.file_id)
            .await?;

        println!(
            "{}",
            format!(
                "🎉 Processing completed for {}: Total time: {:?}",
                self.file_name, self.processing_time
            )
            .bright_green()
            .bold()
        );

        Ok(())
    }

    pub async fn fail_with_error(&mut self, error: String) -> Result<(), redis::RedisError> {
        self.status = ProcessingStatus::Failure;
        self.processing_time = self.start_time.elapsed();
        self.progress_manager
            .fail_processing(&self.file_id, error)
            .await?;

        Ok(())
    }

    pub fn set_total_products(&mut self, total_products: usize) {
        self.total_products = total_products;
    }

    pub async fn mark_row_processed(&mut self, row_number: usize, processing_start: Instant) {
        let processing_time = processing_start.elapsed();
        let processed_at = Instant::now();

        if self.processed_rows.insert(row_number) {
            self.row_details.insert(
                row_number,
                ProcessedRowInfo {
                    row_number,
                    processing_time,
                    processed_at,
                },
            );

            // Update progress
            self.progress_manager
                .increment_progress(&self.file_id, true)
                .await
                .unwrap_or_else(|e| {
                    println!("{}", format!("Failed to update progress: {}", e).red());
                });
        }
    }

    pub async fn mark_product_processed(
        &mut self,
        sku: String,
        product_type: ProductProcessType,
        row_number: usize,
        processing_start: Instant,
    ) {
        let processing_time = processing_start.elapsed();
        let processed_at = Instant::now();

        // Update product counters
        match product_type {
            ProductProcessType::Parent => self.parent_products += 1,
            ProductProcessType::Child => self.child_products += 1,
            ProductProcessType::Standalone => {
                self.parent_products += 1;
            }
        }

        self.product_details.insert(
            sku.clone(),
            ProcessedProductInfo {
                sku: sku.clone(),
                product_type: product_type.clone(),
                row_number,
                processing_time,
                processed_at,
            },
        );

        self.mark_row_processed(row_number, processing_start).await;

        println!(
            "{}",
            format!(
                "✅ Processed {}: {} ({:?}) in {:?}",
                match product_type {
                    ProductProcessType::Parent => "Product",
                    ProductProcessType::Child => "Variation",
                    ProductProcessType::Standalone => "Standalone",
                },
                sku,
                processing_time,
                processing_time
            )
            .green()
        );
    }

    // Getter methods for analytics
    pub fn get_processed_row_count(&self) -> usize {
        self.processed_rows.len()
    }

    pub fn get_processed_product_count(&self) -> usize {
        self.product_details.len()
    }

    pub fn is_row_processed(&self, row_number: usize) -> bool {
        self.processed_rows.contains(&row_number)
    }

    pub fn get_row_processing_time(&self, row_number: usize) -> Option<Duration> {
        self.row_details
            .get(&row_number)
            .map(|info| info.processing_time)
    }

    pub fn get_product_processing_time(&self, sku: &str) -> Option<Duration> {
        self.product_details
            .get(sku)
            .map(|info| info.processing_time)
    }

    pub fn get_average_row_processing_time(&self) -> Duration {
        if self.row_details.is_empty() {
            return Duration::default();
        }

        let total_time: Duration = self
            .row_details
            .values()
            .map(|info| info.processing_time)
            .sum();

        total_time / self.row_details.len() as u32
    }

    pub fn get_slowest_processed_rows(&self, limit: usize) -> Vec<(usize, Duration)> {
        let mut rows: Vec<_> = self
            .row_details
            .iter()
            .map(|(row_num, info)| (*row_num, info.processing_time))
            .collect();

        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows.truncate(limit);
        rows
    }

    pub fn get_processing_analytics(&self) -> ProcessingAnalytics {
        ProcessingAnalytics {
            total_rows_processed: self.get_processed_row_count(),
            total_products_processed: self.get_processed_product_count(),
            average_row_time: self.get_average_row_processing_time(),
            parent_products: self.parent_products,
            child_products: self.child_products,
            failed_rows: self.failed_row.len(),
            success_rate: if self.total_row > 0 {
                (self.get_processed_row_count() as f64 / self.total_row as f64) * 100.0
            } else {
                0.0
            },
            processing_duration: self.processing_time,
        }
    }
}

// In processing_result.rs - Update these methods

impl ProcessingResult {
    // REPLACE the existing mark_failure method
    pub async fn mark_failure(&mut self, row_number: usize, reason: String, sku: Option<String>) {
        self.status = ProcessingStatus::Failure;

        // Add to progress manager with detailed info
        self.progress_manager
            .add_failed_row(&self.file_id, row_number, reason.clone())
            .await
            .unwrap_or_else(|e| {
                println!(
                    "{}",
                    format!("Failed to update failed row progress: {}", e).red()
                );
            });

        if let Some(product_sku) = sku.clone() {
            self.progress_manager
                .add_failed_product(&self.file_id, product_sku, reason.clone())
                .await
                .unwrap_or_else(|e| {
                    println!(
                        "{}",
                        format!("Failed to update failed product progress: {}", e).red()
                    );
                });
        }

        // Keep existing failed_row for backward compatibility if needed
        self.failed_row.push(FailedRow {
            row_number,
            reason: reason.clone(),
            sku: sku.clone(),
        });

        println!(
            "{}",
            format!(
                "❌ Row {} failed: {} (SKU: {:?})",
                row_number,
                reason,
                sku.unwrap_or_default()
            )
            .red()
        );
    }
}

impl ProcessingResult {
    // NEW BATCH METHODS
    pub async fn start_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        products_in_batch: usize,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::BatchStarted {
            batch_number,
            total_batches,
            products_in_batch,
        })
        .await
    }

    pub async fn pause_for_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        delay_minutes: u32,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::BatchPaused {
            batch_number,
            total_batches,
            delay_minutes,
        })
        .await
    }

    pub async fn complete_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        successful_products: usize,
        failed_products: usize,
    ) -> Result<(), redis::RedisError> {
        self.update_stage(ProcessingStage::BatchCompleted {
            batch_number,
            total_batches,
            successful_products,
            failed_products,
        })
        .await
    }
}
