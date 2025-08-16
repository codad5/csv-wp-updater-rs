use crate::helper::file_helper::get_upload_path;
use crate::helper::{calculate_hash, clean_string};
use crate::libs::processing_result::FailedRow;
use crate::libs::redis::FileProcessingManager;
use crate::types::csv_field_woo_mapper::{AttributeMapping, WordPressFieldMapping};
use crate::types::woocommerce::{
    woo_build_product, ProductAttribute, ProductVariation, WooCommerceProduct, WooProduct,
};
use crate::worker::NewFileProcessQueue;
use colored::*;
use csv::Reader;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Instant;

use tokio::sync::Semaphore;

use crate::libs::{
    processing_result::{ProcessingResult, ProductProcessType},
    progress_manager::{ProcessingStage, ProgressManager},
};

// create our own custom status enum for either success or failure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct GroupParentResult {
    group: Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
    failed: Vec<FailedRow>,
}

impl GroupParentResult {
    pub fn new() -> Self {
        GroupParentResult {
            group: Vec::new(),
            failed: Vec::new(),
        }
    }
    pub fn get_group(&self) -> &Vec<(WooCommerceProduct, Vec<ProductVariation>)> {
        &self.group
    }
    pub fn get_failed(&self) -> &Vec<FailedRow> {
        &self.failed
    }

    pub fn get_total(&self) -> usize {
        self.group.len() + self.failed.len()
    }

    pub fn get_total_products(&self) -> usize {
        let parent_count = self.group.len();
        let child_count: usize = self.group.iter().map(|(_, children)| children.len()).sum();
        parent_count + child_count
    }

    pub fn add_group(&mut self, parent: WooCommerceProduct, children: Vec<ProductVariation>) {
        self.group.push((parent, children));
    }

    pub fn add_many_group(&mut self, groups: Vec<(WooCommerceProduct, Vec<ProductVariation>)>) {
        self.group.extend(groups);
    }

    pub fn add_failed_row(&mut self, row_number: usize, reason: String) {
        self.failed.push(FailedRow {
            row_number,
            reason,
            sku: None,
        });
    }
}

// New structure to track individual row processing details
#[derive(Debug, Clone)]
pub struct ProcessedRowInfo {
    pub row_number: usize,
    pub processing_time: Duration,
    pub processed_at: Instant,
}

// New structure to track product processing details
#[derive(Debug, Clone)]
pub struct ProcessedProductInfo {
    pub sku: String,
    pub product_type: ProductProcessType, // Parent, Child, or Standalone
    pub row_number: usize,
    pub processing_time: Duration,
    pub processed_at: Instant,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct BatchProductRequest {
    pub create: Vec<WooCommerceProduct>,
    pub update: Vec<WooCommerceProduct>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct BatchProductResponse {
    pub create: Vec<WooCommerceProduct>,
    pub update: Vec<WooCommerceProduct>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ProcessingProgress {
    total_rows: usize,
    processed_rows: usize,
    successful_rows: usize,
    failed_rows: usize,
    new_entries: usize,
}
#[derive(Debug, Clone)]
struct WooCommerceProcessor {
    woocommerce_client: Arc<Client>,
    redis_client: redis::Client,
    base_url: String,
    consumer_key: String,
    consumer_secret: String,
    result: Arc<RwLock<ProcessingResult>>,
    batch_size: usize,
    batch_delay_minutes: u32,
    dry_run: bool,
}

impl WooCommerceProcessor {
    async fn new(
        base_url: String,
        consumer_key: String,
        consumer_secret: String,
        file_path: String,
        file_id: String,
        total_rows: usize,
        batch_size: usize,
        batch_delay_minutes: u32,
        dry_run: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let woocommerce_client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_else(|_| Client::new());

        let redis_client =
            redis::Client::open("redis://redis:6379/").expect("Failed to create Redis client");

        let base_url = if base_url.ends_with('/') {
            base_url.trim_end_matches('/').to_string()
        } else {
            base_url
        };

        // Initialize ProcessingResult directly in new()
        let processing_result =
            ProcessingResult::new(file_path.clone(), file_id, total_rows).await?;

        println!(
            "{}",
            format!("📈 Initialized processing for: {}", file_path).bright_green()
        );

        Ok(WooCommerceProcessor {
            woocommerce_client: Arc::new(woocommerce_client),
            redis_client,
            base_url,
            consumer_key,
            consumer_secret,
            result: Arc::new(RwLock::new(processing_result)), // RwLock instead of Mutex
            batch_size,
            batch_delay_minutes,
            dry_run,
        })
    }

    // READ OPERATIONS - Multiple concurrent access allowed
    // Convenience methods for updating progress
    async fn set_csv_parsing_complete(&self) -> Result<(), redis::RedisError> {
        self.result.read().await.set_csv_parsing_complete().await
    }

    async fn set_grouping_products(&self) -> Result<(), redis::RedisError> {
        self.result.read().await.set_grouping_products().await
    }

    async fn set_grouping_complete(
        &self,
        total_parents: usize,
        total_products: usize,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .set_grouping_complete(total_parents, total_products)
            .await
    }

    async fn start_processing_product(
        &self,
        sku: String,
        current: usize,
        total: usize,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .start_processing_product(sku, current, total)
            .await
    }

    async fn start_processing_variation(
        &self,
        sku: String,
        parent_sku: String,
        current: usize,
        total: usize,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .start_processing_variation(sku, parent_sku, current, total)
            .await
    }

    async fn finalizing(&self) -> Result<(), redis::RedisError> {
        self.result.read().await.finalizing().await
    }

    async fn complete_processing(&self) -> Result<(), redis::RedisError> {
        self.result.write().await.complete().await
    }

    async fn fail_processing(&self, error: String) -> Result<(), redis::RedisError> {
        self.result.write().await.fail_with_error(error).await
    }

    async fn mark_product_processed(
        &self,
        sku: String,
        product_type: ProductProcessType,
        row_number: usize,
        processing_start: Instant,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.result
            .write()
            .await
            .mark_product_processed(sku, product_type, row_number, processing_start.into())
            .await;
        Ok(())
    }

    async fn mark_failure(
        &self,
        row_number: usize,
        reason: String,
        sku: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.result
            .write()
            .await
            .mark_failure(row_number, reason, sku)
            .await;
        Ok(())
    }

    async fn set_total_products(
        &self,
        total: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.result.write().await.set_total_products(total);
        Ok(())
    }

    async fn process_csv(
        self: &Arc<Self>,
        file_path: &str,
        field_mapping: &WordPressFieldMapping,
        setting: &NewFileProcessQueue,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        //IMPORTANT DOCs: HOW this works
        //TODO: HOW this works
        //IMPORTANT DOCS: HOW CSV PROCESSING WORKS
        //
        // This function processes CSV files containing WooCommerce product data in a highly concurrent manner.
        // The workflow follows these key steps:
        //
        // 1. FILE SETUP & VALIDATION:
        //    - Reads CSV from the specified file path
        //    - Counts total rows and validates processing parameters (start_row, row_count)
        //    - Limits processing to maximum 40,000 rows per batch for performance
        //    - Creates file_id from filename (without extension) for tracking progress
        //
        // 2. FIELD MAPPING PREPARATION:
        //    - Uses WordPressFieldMapping to create reverse mappings for CSV headers
        //    - Cleans header names and maps them to internal product fields
        //    - Handles both regular fields and attribute mappings
        //
        // 3. PRODUCT GROUPING:
        //    - Calls `group_products_by_parent()` to organize CSV rows into parent-child relationships
        //    - Groups variations under their parent products: Vec<(parent_product, Vec<child_variations>)>
        //    - This allows processing entire product families together
        //
        // 4. CONCURRENT PROCESSING:
        //    - Uses semaphore-based concurrency control (100-300 concurrent tasks based on file size)
        //    - For each product group:
        //      a) Spawns a parent task that processes the main product
        //      b) Within each parent task, spawns child tasks for all variations
        //      c) Uses Redis for caching and duplicate detection
        //
        // 5. DUPLICATE DETECTION VIA SHA HASHING:
        //    - Before processing any product, checks Redis for existing data using SHA comparison
        //    - WHAT WE STORE IN SHA:
        //      * Key: "products:sha:{sku}"
        //      * Value: SHA hash of the ORIGINAL CSV product data (before API processing)
        //    - COMPARISON PROCESS:
        //      * Takes current CSV row data, serializes to JSON, calculates SHA hash
        //      * Compares with stored SHA hash from previous processing
        //      * If hashes match → product hasn't changed, skip API call and reuse existing ID
        //      * If hashes differ → product data has changed, process normally
        //    - PURPOSE: Avoids redundant API calls to WooCommerce when CSV data hasn't changed
        //
        // 6. REDIS CACHING STRATEGY:
        //    - "products:{sku}" → Complete processed product data (after API response)
        //    - "products:sha:{sku}" → SHA hash of original CSV data (for change detection)
        //    - "products:id:{sku}" → Product ID from WooCommerce API (for quick lookups)
        //
        // 7. ERROR HANDLING & PROGRESS TRACKING:
        //    - FileProcessingManager tracks overall progress and handles failures
        //    - Each task updates progress counters (successful_rows, failed_rows, processed_rows)
        //    - Redis connection errors are handled gracefully with progress updates
        //    - Failed tasks don't block other concurrent processing
        //
        // 8. PROCESSING FLOW:
        //    - Parent products are created/updated first via `handle_main_product()`
        //    - Child variations are then processed via `handle_variation_product()` using parent_id
        //    - All operations are async and concurrent within semaphore limits
        //    - Progress is tracked and reported throughout the process
        //
        // PERFORMANCE CONSIDERATIONS:
        // - Concurrency is dynamically set based on file size (total_rows/10, clamped 100-300)
        // - SHA-based caching dramatically reduces API calls for unchanged data
        // - Semaphore prevents overwhelming the WooCommerce API or Redis
        // - Batch processing with configurable start_row and row_count for large files
        // File id is file path without ext
        let start_row: u32 = setting.start_row;
        let no_of_rows: u32 = setting.row_count;
        let new_product = setting.is_new_upload;

        println!(
            "{}",
            format!("🚀 Starting CSV processing for file: {}", file_path)
                .bright_blue()
                .bold()
        );

        // Count total rows first
        let mut rdr = Reader::from_path(get_upload_path(file_path))?;
        let total_row_count: u32 = rdr.records().count().try_into().unwrap();

        let rows_to_process = if no_of_rows == 0 {
            total_row_count - start_row
        } else {
            no_of_rows.min(total_row_count - start_row)
        };
        let rows_to_process = rows_to_process.min(40_000);

        println!(
            "Processing from row {} for {} rows",
            start_row, rows_to_process
        );

        // Set CSV parsing stage
        self.set_csv_parsing_complete().await?;

        // Reset reader and prepare data
        let mut rdr = Reader::from_path(get_upload_path(file_path))?;
        let headers = rdr.headers()?.clone();

        let reverse_mapping = field_mapping.get_reverse_mapping();
        let reverse_mapping: HashMap<String, String> = reverse_mapping
            .iter()
            .map(|(k, v)| (clean_string(k), v.clone()))
            .collect();

        let reverse_attribute_mapping = field_mapping.get_inverted_attribute();
        let reverse_attribute_mapping: HashMap<String, AttributeMapping> =
            reverse_attribute_mapping
                .iter()
                .map(|(k, v)| (clean_string(k), v.clone()))
                .collect();

        // Set grouping stage
        self.set_grouping_products().await?;

        let record_vec: Vec<Result<csv::StringRecord, csv::Error>> = rdr
            .records()
            .skip(start_row as usize)
            .take(rows_to_process as usize)
            .collect();

        let grouped_products = Self::group_products_by_parent(
            record_vec,
            &headers,
            &reverse_mapping,
            &reverse_attribute_mapping,
        )?;

        // Update progress after grouping
        self.set_grouping_complete(
            grouped_products.get_group().len(),
            grouped_products.get_total_products(),
        )
        .await?;

        self.set_total_products(grouped_products.get_total_products())
            .await?;

        // Process standalone products
        let (products_with_children, products_without_children): (Vec<_>, Vec<_>) =
            grouped_products
                .get_group()
                .clone()
                .into_iter()
                .partition(|(_, children)| !children.is_empty());

        // Process standalone products in batches
        if !products_without_children.is_empty() {
            self.process_standalone_products(products_without_children, new_product)
                .await?;
        }

        // Process products with variations
        self.process_products_with_variations(products_with_children, new_product)
            .await?;

        // Finalize processing
        self.finalizing().await?;
        self.complete_processing().await?;

        println!(
            "{}",
            format!("✨ Processing completed successfully for: {}", file_path)
                .bright_green()
                .bold()
        );
        Ok(())
    }

    async fn compare_product_last_instance(
        product: &WooCommerceProduct,
        redis_conn: &mut MultiplexedConnection,
    ) -> Option<String> {
        if let Ok(Some(product_json_sha)) = redis_conn
            .hget::<_, _, Option<String>>("products:sha", &product.sku)
            .await
        {
            let found_product_json = serde_json::to_string(&product).unwrap_or("{}".to_string());
            if calculate_hash(found_product_json) == product_json_sha {
                println!("Sha matches going to the next");
                if let Ok(Some(product_json_id)) = redis_conn
                    .hget::<_, _, Option<String>>("products:id", &product.sku)
                    .await
                {
                    return Some(product_json_id); // Found in Redis, return it
                }
            }
            // println!("SKU mismatch: expected {}, found {}", sku, product.sku);
        }
        None
    }
    async fn compare_product_variation_last_instance(
        product: &ProductVariation,
        redis_conn: &mut MultiplexedConnection,
    ) -> Option<String> {
        if let Ok(Some(product_json_sha)) = redis_conn
            .hget::<_, _, Option<String>>("products:sha", &product.sku)
            .await
        {
            let found_product_json = serde_json::to_string(&product).unwrap_or("{}".to_string());
            if calculate_hash(found_product_json) == product_json_sha {
                println!("Sha matches going to the next");
                if let Ok(Some(product_json_id)) = redis_conn
                    .hget::<_, _, Option<String>>("products:id", &product.sku)
                    .await
                {
                    return Some(product_json_id); // Found in Redis, return it
                }
            }
            // println!("SKU mismatch: expected {}, found {}", sku, product.sku);
        }
        None
    }

    async fn process_standalone_products(
        self: &Arc<Self>,
        standalone_products_data: Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
        new_product: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let standalone_products: Vec<WooCommerceProduct> = standalone_products_data
            .into_iter()
            .map(|(parent, _)| parent)
            .collect();

        // let batch_size = 50;
        let batches: Vec<Vec<WooCommerceProduct>> = standalone_products
            .chunks(self.batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        println!(
            "{}",
            format!(
                "📦 Processing {} standalone products in {} batches",
                standalone_products.len(),
                batches.len()
            )
            .cyan()
        );

        for (batch_index, batch) in batches.clone().into_iter().enumerate() {
            let batch_number = batch_index + 1;
            let batch_len = batch.len();
            // Add delay before processing (except for first batch)
            if batch_index > 0 && self.batch_delay_minutes > 0 {
                println!(
                    "{}",
                    format!(
                        "⏳ Waiting {} minutes before processing batch {}...",
                        self.batch_delay_minutes,
                        batch_index + 1
                    )
                    .yellow()
                );
                self.pause_for_batch(batch_number - 1, batches.len(), self.batch_delay_minutes)
                    .await?;
                tokio::time::sleep(Duration::from_secs(self.batch_delay_minutes as u64 * 60)).await;
            }

            self.start_batch(batch_number, batches.len(), batch_len)
                .await?;

            let batch_len = batch.len();

            for i in 0..batch_len {
                // Update progress for this batch
                self.start_processing_product(
                    format!("Batch {}", (batch_index * self.batch_size) + i + 1),
                    (batch_index * self.batch_size) + i + 1,
                    standalone_products.len(),
                )
                .await?;
            }

            let batch_tuple = if new_product {
                (batch, Vec::new())
            } else {
                (Vec::new(), batch)
            };

            let mut successful = 0;
            let mut failed = 0;

            match self.batch_update_products(batch_tuple).await {
                Ok(_) => {
                    successful = batch_len;
                    println!(
                        "{}",
                        format!(
                            "✅ Batch {} processed successfully ({} products)",
                            batch_index + 1,
                            batch_len
                        )
                        .green()
                    );
                }
                Err(e) => {
                    failed = batch_len;
                    println!(
                        "{}",
                        format!("❌ Batch {} processing failed: {:?}", batch_index + 1, e).red()
                    );

                    self.mark_failure(0, format!("Batch {} failed: {}", batch_index + 1, e), None)
                        .await?;
                }
            }
            self.complete_batch(batch_number, batches.len(), successful, failed)
                .await?;
        }

        Ok(())
    }

    async fn process_products_with_variations(
        self: &Arc<Self>,
        products_with_children: Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
        new_product: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let total_products = products_with_children.len();
        let mut processed_products = 0;
        let semaphore = Arc::new(Semaphore::new(100));
        let mut parent_futures: Vec<(usize, tokio::task::JoinHandle<()>)> = Vec::new();

        println!(
            "{}",
            format!("👨‍👩‍👧‍👦 Processing {} products with variations", total_products).blue()
        );

        for (index, (parent, children)) in products_with_children.into_iter().enumerate() {
            processed_products += 1;
            // let current_index = index + 1;
            let total_products_in_group = 1 + children.len(); // parent + children count
                                                              // Update progress

            let semaphore_clone = Arc::clone(&semaphore);
            let self_clone = Arc::clone(self);

            let parent_task = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                let start_time = Instant::now();

                self_clone
                    .start_processing_product(
                        parent.sku.clone(),
                        processed_products,
                        total_products,
                    )
                    .await
                    .unwrap_or(());

                if let Ok(mut redis_conn) = self_clone
                    .redis_client
                    .get_multiplexed_async_connection()
                    .await
                {
                    // Check for existing product first
                    let mut parent_id = parent.id.clone();
                    if let Some(prod_id) =
                        Self::compare_product_last_instance(&parent, &mut redis_conn).await
                    {
                        parent_id = prod_id;
                        println!(
                            "{}",
                            format!("♻️ Reusing existing parent: {}", parent.sku).yellow()
                        );
                    } else {
                        // Process parent product
                        match self_clone
                            .handle_main_product(&parent, &mut redis_conn, &new_product)
                            .await
                        {
                            Ok(updated_parent) => {
                                parent_id = updated_parent.id.clone();

                                // Cache the result
                                let json_body = serde_json::to_string(&updated_parent)
                                    .unwrap_or("{}".to_string());
                                let _: () = redis_conn
                                    .hset("products", &updated_parent.sku, json_body)
                                    .await
                                    .unwrap_or(());
                                let saved_json_body =
                                    serde_json::to_string(&parent).unwrap_or("{}".to_string());
                                let _: () = redis_conn
                                    .hset(
                                        "products:sha",
                                        &updated_parent.sku,
                                        calculate_hash(saved_json_body),
                                    )
                                    .await
                                    .unwrap_or(());
                                let _: () = redis_conn
                                    .hset(
                                        "products:id",
                                        &updated_parent.sku,
                                        updated_parent.id.clone(),
                                    )
                                    .await
                                    .unwrap_or(());

                                // Mark parent as processed
                                self_clone
                                    .mark_product_processed(
                                        parent.sku.clone(),
                                        ProductProcessType::Parent,
                                        processed_products,
                                        start_time,
                                    )
                                    .await
                                    .unwrap_or(());

                                println!(
                                    "{}",
                                    format!("✅ Parent processed: {}", parent.sku).green()
                                );
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    format!("❌ Parent failed: {} - {}", parent.sku, e).red()
                                );
                                self_clone
                                    .mark_failure(
                                        processed_products,
                                        format!("Parent processing failed: {}", e),
                                        Some(parent.sku.clone()),
                                    )
                                    .await
                                    .unwrap_or(());
                                return;
                            }
                        }
                    }

                    // Process child variations
                    for (child_index, child) in children.iter().enumerate() {
                        processed_products += 1;
                        let child_start = Instant::now();

                        // Update progress for variation
                        self_clone
                            .start_processing_variation(
                                child.sku.clone(),
                                parent.sku.clone(),
                                child_index + 1,
                                children.len(),
                            )
                            .await
                            .unwrap_or(());

                        // Check for existing variation first
                        if let Some(_existing_id) =
                            Self::compare_product_variation_last_instance(child, &mut redis_conn)
                                .await
                        {
                            println!(
                                "{}",
                                format!("♻️ Reusing existing variation: {}", child.sku).yellow()
                            );
                            continue;
                        }

                        match self_clone
                            .handle_variation_product(
                                child,
                                &parent_id,
                                &mut redis_conn,
                                &new_product,
                            )
                            .await
                        {
                            Ok(updated_child) => {
                                // Cache the result
                                let json_body = serde_json::to_string(&updated_child)
                                    .unwrap_or("{}".to_string());
                                let _: () = redis_conn
                                    .hset("products", &updated_child.sku, json_body)
                                    .await
                                    .unwrap_or(());
                                let saved_json_body =
                                    serde_json::to_string(&child).unwrap_or("{}".to_string());
                                let _: () = redis_conn
                                    .hset(
                                        "products:sha",
                                        &updated_child.sku,
                                        calculate_hash(saved_json_body),
                                    )
                                    .await
                                    .unwrap_or(());
                                let _: () = redis_conn
                                    .hset(
                                        "products:id",
                                        &updated_child.sku,
                                        updated_child.id.clone(),
                                    )
                                    .await
                                    .unwrap_or(());

                                // Mark variation as processed
                                self_clone
                                    .mark_product_processed(
                                        child.sku.clone(),
                                        ProductProcessType::Child,
                                        processed_products,
                                        child_start,
                                    )
                                    .await
                                    .unwrap_or(());

                                println!(
                                    "{}",
                                    format!(
                                        "✅ Variation processed: {} ({}/{})",
                                        child.sku,
                                        child_index + 1,
                                        children.len()
                                    )
                                    .green()
                                );
                            }
                            Err(e) => {
                                println!(
                                    "{}",
                                    format!("❌ Variation failed: {} - {}", child.sku, e).red()
                                );
                                self_clone
                                    .mark_failure(
                                        0,
                                        format!("Variation processing failed: {}", e),
                                        Some(child.sku.clone()),
                                    )
                                    .await
                                    .unwrap_or(());
                            }
                        }
                    }
                }
            });

            parent_futures.push((total_products_in_group, parent_task));
        }

        let mut current_batch_product_count = 0;
        let batch_size = self.batch_size as usize;
        let delay_minutes = self.batch_delay_minutes;
        // Wait for all tasks to complete
        for (product_count_in_group, task) in parent_futures {
            // Check if adding this group would exceed batch size
            if current_batch_product_count + product_count_in_group > batch_size
                && current_batch_product_count > 0
            {
                // Delay before processing next batch
                if delay_minutes > 0 {
                    println!(
                        "{}",
                        format!(
                    "⏳ Batch complete ({} products). Waiting {} minutes before next batch...",
                    current_batch_product_count, delay_minutes
                )
                        .yellow()
                    );
                    tokio::time::sleep(Duration::from_secs(delay_minutes as u64 * 60)).await;
                }
                current_batch_product_count = 0; // Reset counter
            }

            // Execute the task
            if let Err(e) = task.await {
                println!("{}", format!("Task execution error: {:?}", e).red());
                self.mark_failure(0, format!("Task execution failed: {}", e), None)
                    .await?;
            }

            current_batch_product_count += product_count_in_group;
        }

        Ok(())
    }

    async fn handle_main_product(
        &self,
        product: &WooCommerceProduct,
        redis_conn: &mut MultiplexedConnection,
        new_product: &bool,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
        // if its parent id is empty then its a main product
        if !product.parent.is_empty() && product.parent != product.sku {
            return Err("Product is not a main product".into());
        }

        let exists = self
            .get_or_fetch_product(redis_conn, &product, new_product)
            .await;
        let mut new_product_update = product.clone();
        println!("new product update: {:?}", new_product_update);

        if let Some(found_product) = exists {
            // if let Ok(Some(product_json_sha)) = redis_conn.hget::<_, _, Option<String>>("products:sha", &found_product.sku).await {
            //     println!("Product sha found in Redis: {:?}", product);
            //     // Check if the SKU matches
            //     let found_product_json = serde_json::to_string(&found_product).unwrap_or("{}".to_string());
            //     if calculate_hash(found_product_json)  === product_json_sha {
            //         return Ok(found_product); // Found in Redis, return it
            //     }
            //     println!("SKU mismatch: expected {}, found {}", sku, product.sku);
            // }

            // merge the new product update with the existing
            // check if there is any difference between the merged and the new product update, if any diff call the update method to update through the api
            // if no change skip

            // Check core fields that would require an update
            if found_product.has_changed(&new_product_update) {
                let update_prod = found_product.merge(&new_product_update);

                // print before and after the merge
                println!(
                    "Product before merge: {:?} \nProduct after merge: {:?}",
                    found_product, update_prod
                );
                let update_prod = self.update_product(&update_prod).await;
                match update_prod {
                    Ok(p) => {
                        println!("Product updated successfully: {:?}", p);
                        new_product_update = p;
                        // let mut progress = progress_clone.lock().await;
                        // progress.successful_rows += 1;
                        // progress.processed_rows += 1;
                    }
                    Err(e) => {
                        return Err(format!("Error updating product: {:?}", e).into());
                        // let mut progress = progress_clone.lock().await;
                        // progress.failed_rows += 1;
                        // progress.processed_rows += 1;
                    }
                }
            } else {
                println!(
                    "No changes detected for product: {:?}",
                    new_product_update.sku
                );
            }
        } else {
            // if not found create a new product but an ID must exist

            // Ensure required fields are present
            if let Err(e) = new_product_update.validate() {
                let error_msg = format!(
                    "Missing required fields for product creation: {:?} in {:?}",
                    e, new_product_update
                );
                return Err(error_msg.into());
            }

            let mut create_new_product = new_product_update.clone();
            create_new_product.id = String::new();
            let new_product = self.create_product(&create_new_product).await;
            match new_product {
                Ok(p) => {
                    new_product_update = p;
                    // let mut progress = progress_clone.lock().await;
                    // progress.successful_rows += 1;
                    // progress.processed_rows += 1;
                }
                Err(e) => {
                    return Err(format!("Error creating product: {:?}", e).into());
                }
            }
        }

        if new_product_update.validate().is_err() {
            return Err(format!("Product validation failed: {:?}", new_product_update).into());
        }

        Ok(new_product_update)
    }

    async fn handle_variation_product(
        &self,
        product: &ProductVariation,
        parent_id: &str,
        redis_conn: &mut MultiplexedConnection,
        new_product: &bool,
    ) -> Result<ProductVariation, Box<dyn std::error::Error + Send + Sync>> {
        // if its parent id is empty then its a main product
        if product.parent.is_empty() {
            return Err("Product is a main product".into());
        }

        let exists = self
            .get_or_fetch_product_variation(
                redis_conn,
                &product,
                parent_id.to_string(),
                new_product,
            )
            .await;
        let mut new_product_update = product.clone();

        if let Some(found_product) = exists {
            // merge the new product update with the existing
            // check if there is any difference between the merged and the new product update, if any diff call the update method to update through the api
            // if no change skip

            // Check core fields that would require an update
            if found_product.has_changed(&new_product_update) {
                let update_prod = found_product.merge(&new_product_update);
                let update_prod = self.update_product_variation(&update_prod, parent_id).await;
                match update_prod {
                    Ok(p) => {
                        new_product_update = p;
                        // let mut progress = progress_clone.lock().await;
                        // progress.successful_rows += 1;
                        // progress.processed_rows += 1;
                    }
                    Err(e) => {
                        return Err(format!("Error updating product: {:?}", e).into());
                        // let mut progress = progress_clone.lock().await;
                        // progress.failed_rows += 1;
                        // progress.processed_rows += 1;
                    }
                }
            }
        } else {
            // if not found create a new product but an ID must exist

            // Ensure required fields are present
            if let Err(e) = new_product_update.validate() {
                let error_msg = format!(
                    "Missing required fields for product creation: {:?} in {:?}",
                    e, new_product_update
                );
                return Err(error_msg.into());
            }

            let mut create_new_product = new_product_update.clone();
            create_new_product.id = String::new();
            let new_product = self
                .create_product_variation(&create_new_product, parent_id)
                .await;
            match new_product {
                Ok(p) => {
                    new_product_update = p;
                    // let mut progress = progress_clone.lock().await;
                    // progress.successful_rows += 1;
                    // progress.processed_rows += 1;
                }
                Err(e) => {
                    return Err(format!("Error creating product: {:?}", e).into());
                    // let mut progress = progress_clone.lock().await;
                    // progress.failed_rows += 1;
                    // progress.processed_rows += 1;
                }
            }
        }

        Ok(new_product_update)
    }

    fn group_products_by_parent(
        records: Vec<Result<csv::StringRecord, csv::Error>>,
        headers: &csv::StringRecord,
        reverse_mapping: &HashMap<String, String>,
        attribute_reverse: &HashMap<String, AttributeMapping>,
    ) -> Result<GroupParentResult, Box<dyn std::error::Error + Send + Sync>> {
        // HashMap to store parent SKU/ID -> vector of children
        let mut parent_children_map: std::collections::HashMap<String, Vec<ProductVariation>> =
            std::collections::HashMap::new();

        // Vector to store parent products
        let mut parent_products: Vec<WooCommerceProduct> = Vec::new();
        let mut result = GroupParentResult::new();

        println!(
            "\x1b[38;5;82mReverse Mapping Debug Info: {:?}\x1b[0m",
            reverse_mapping
        );
        println!(
            "\x1b[38;5;196mAttribute Reverse Mapping Debug Info: {:?}\x1b[0m",
            attribute_reverse
        );

        let mut row_number = 0;

        // Process each record once - O(n) single pass
        for record_result in records.iter() {
            row_number += 1;
            let record = match record_result {
                Ok(record) => record,
                Err(e) => {
                    result.add_failed_row(row_number, format!("Error processing record: {:?}", e));
                    println!("Error processing record: {:?}", e);
                    continue;
                }
            };

            // Create a HashMap from the record using the provided approach
            let row_map: HashMap<String, String> = headers
                .iter()
                .zip(record.iter())
                .map(|(h, v)| {
                    (
                        reverse_mapping
                            .get(h)
                            .unwrap_or(&"".to_string())
                            .to_lowercase(),
                        v.to_string(),
                    )
                })
                .collect();

            let attribute_row_map: HashMap<String, AttributeMapping> = headers
                .iter()
                .zip(record.iter())
                .map(|(h, v)| {
                    let binding = AttributeMapping::default();
                    let vad = attribute_reverse.get(h).unwrap_or(&binding);
                    (
                        vad.clone().column.to_lowercase(),
                        AttributeMapping {
                            column: v.to_string(),
                            variable: vad.variable.clone(),
                        },
                    )
                })
                .collect();

            println!(
                "{}",
                format!("Product HashMap Debug Info: {:?}", row_map).green(),
            );
            println!(
                "{}",
                format!("Product HashMap Debug Info: {:?}", attribute_row_map).blue()
            );

            // Build product from row_map using the new woo_build_product function
            match woo_build_product(&row_map, &attribute_row_map) {
                Some(WooProduct::Product(product)) => {
                    // This is a parent or standalone product
                    // Add to parent products vector
                    parent_products.push(product.clone());
                    println!("Parent Product found {:?}", product);

                    // Ensure there's an entry in the map for this parent
                    if !parent_children_map.contains_key(&product.sku) {
                        parent_children_map.insert(product.sku.clone(), Vec::new());
                    }
                }
                Some(WooProduct::Variation(variation)) => {
                    // This is a child product (variation)
                    // Add to the parent's children vector in the map
                    println!("Child Product found {:?}", variation);
                    parent_children_map
                        .entry(variation.parent.clone())
                        .or_insert_with(Vec::new)
                        .push(variation);
                }
                None => {
                    result.add_failed_row(
                        row_number,
                        "Error building product from record".to_string(),
                    );
                    println!("Error building product from record");
                    continue;
                }
            }
        }

        // Create the final result structure - O(p) where p is number of parents
        let groups_result: Vec<(WooCommerceProduct, Vec<ProductVariation>)> = parent_products
            .into_iter()
            .map(|mut parent| {
                let mut children = parent_children_map
                    .remove(&parent.sku)
                    .unwrap_or_else(Vec::new);
                // let parent_id = parent.id.clone();
                let pa_attribute_binding = parent.get_attribute_mut();

                // For each child/variation
                for child in &mut children {
                    // For each attribute in the child
                    // let parent_ = parent.clone();
                    // child.set_parent(&parent_id);
                    for child_attr in &child.get_attribute() {
                        // Try to find a matching attribute in the parent by name
                        let parent_attr = pa_attribute_binding
                            .iter_mut()
                            .find(|attr| attr.name == child_attr.name);

                        match parent_attr {
                            Some(attr) => {
                                // If the parent already has this attribute, add the option if it's not already there
                                if !attr.options.contains(&child_attr.option) {
                                    attr.options.push(child_attr.option.clone());
                                }
                            }
                            None => {
                                // If the parent doesn't have this attribute yet, create a new one
                                let new_attr = ProductAttribute::new(
                                    child_attr.name.clone().as_str(),
                                    vec![child_attr.option.clone()],
                                );

                                pa_attribute_binding.push(new_attr);
                            }
                        }
                    }
                }

                (parent, children)
            })
            .collect();

        result.add_many_group(groups_result);
        Ok(result)
    }

    async fn update_product(
        &self,
        product: &WooCommerceProduct,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
        if self.dry_run {
            println!(
                "{}",
                format!(
                    "🔍 DRY RUN: Would update product with SKU: {} and ID: {}",
                    product.sku, product.id
                )
                .bright_cyan()
            );
            return Ok(product.clone());
        }
        let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
        println!(
            "Updating product with sku: {} and JSON body: {}",
            product.sku, json_body
        );
        let res = self
            .woocommerce_client
            .put(&format!(
                "{}/wp-json/wc/v3/products/{}",
                self.base_url, product.id
            ))
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .body(json_body)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        let body = res.text().await?; // Get response as a string
        println!(
            "Response body with sku: {}, update_product: {}",
            product.sku, body
        );
        let products: WooCommerceProduct = serde_json::from_str(&body)?; // Parse JSON manually

        Ok(products)
    }

    async fn update_product_variation(
        &self,
        product: &ProductVariation,
        parent_id: &str,
    ) -> Result<ProductVariation, Box<dyn std::error::Error>> {
        if self.dry_run {
            println!(
                "{}",
                format!(
                    "🔍 DRY RUN: Would update variation with SKU: {} and ID: {} for parent: {}",
                    product.sku, product.id, parent_id
                )
                .bright_cyan()
            );
            return Ok(product.clone());
        }
        let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
        println!(
            "Updating product with id {} variation with sku: {} and JSON body: {}",
            parent_id, product.sku, json_body
        );
        let res = self
            .woocommerce_client
            .put(&format!(
                "{}/wp-json/wc/v3/products/{}/variations/{}",
                self.base_url, parent_id, product.id
            ))
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .body(json_body)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        let body = res.text().await?; // Get response as a string
        println!(
            "Response body with sku: {}, update_product: {}",
            product.sku, body
        );
        let products: ProductVariation = serde_json::from_str(&body)?; // Parse JSON manually

        Ok(products)
    }

    async fn create_product(
        &self,
        product: &WooCommerceProduct,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
        if self.dry_run {
            println!(
                "{}",
                format!("🔍 DRY RUN: Would create product with SKU: {}", product.sku).bright_cyan()
            );
            // Return a mock product with a fake ID for dry run
            let mut mock_product = product.clone();
            mock_product.set_id(format!("dry_run_{}", product.sku));
            return Ok(mock_product);
        }
        // make id empty
        let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
        println!(
            "Creating product with JSON body: {} for product id {} and name {}",
            json_body, product.id, product.name
        );
        let res = self
            .woocommerce_client
            .post(&format!("{}/wp-json/wc/v3/products", self.base_url))
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .body(json_body)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        let body = res.text().await?; // Get response as a string
        println!(
            "Response body from create_product: {} for product id {} and name {}",
            body, product.id, product.name
        );
        let products: WooCommerceProduct = serde_json::from_str(&body)?; // Parse JSON manually

        Ok(products)
    }

    async fn batch_update_products(
        &self,
        products: (Vec<WooCommerceProduct>, Vec<WooCommerceProduct>),
    ) -> Result<BatchProductResponse, Box<dyn std::error::Error + Send + Sync>> {
        if self.dry_run {
            let (create_products, update_products) = products;
            println!(
                "{}",
                format!(
                    "🔍 DRY RUN: Would batch process {} creates and {} updates",
                    create_products.len(),
                    update_products.len()
                )
                .bright_cyan()
            );

            // Return mock response for dry run
            let mock_response = BatchProductResponse {
                create: create_products
                    .into_iter()
                    .map(|mut p| {
                        p.set_id(format!("dry_run_{}", p.sku));
                        p
                    })
                    .collect(),
                update: update_products,
            };
            return Ok(mock_response);
        }
        let (create_products, update_products) = products;

        let batch_request = BatchProductRequest {
            create: create_products,
            update: update_products,
        };

        let json_body = serde_json::to_string(&batch_request)?;
        println!("Batch update JSON body: {}", json_body);

        let res = self
            .woocommerce_client
            .post(&format!("{}/wp-json/wc/v3/products/batch", self.base_url))
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .body(json_body)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body = res.text().await?;
        println!("Response body from batch_update_products: {}", body);

        let response: BatchProductResponse = serde_json::from_str(&body)?;

        Ok(response)
    }

    async fn create_product_variation(
        &self,
        product: &ProductVariation,
        parent_id: &str,
    ) -> Result<ProductVariation, Box<dyn std::error::Error>> {
        if self.dry_run {
            println!(
                "{}",
                format!(
                    "🔍 DRY RUN: Would create variation with SKU: {} for parent ID: {}",
                    product.sku, parent_id
                )
                .bright_cyan()
            );
            let mut mock_variation = product.clone();
            mock_variation.set_id(format!("dry_run_var_{}", product.sku));
            return Ok(mock_variation);
        }

        // amke id empty
        let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
        println!(
            "Creating product variation with JSON body: {} for product id {}",
            json_body, product.id
        );
        let res = self
            .woocommerce_client
            .post(&format!(
                "{}/wp-json/wc/v3/products/{}/variations",
                self.base_url, parent_id
            ))
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .body(json_body)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        let body = res.text().await?; // Get response as a string
        println!(
            "Response body from create_product_variation: {} for product id {}",
            body, product.id,
        );
        let products: ProductVariation = serde_json::from_str(&body)?; // Parse JSON manually

        Ok(products)
    }

    async fn fetch_product_by_sku(
        &self,
        sku: &str,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
        let full_url = format!("{}/wp-json/wc/v3/products?sku={}", self.base_url, sku);
        println!("fetch_product_by_sku : {}", full_url);
        let res = self
            .woocommerce_client
            .get(&full_url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body = res.text().await?; // Get response as a string
        println!(
            "\x1b[38;5;226mResponse body (bright yellow): {}\x1b[0m",
            body
        );
        let products: Vec<WooCommerceProduct> = serde_json::from_str(&body)?; // Parse JSON manually
        println!(
            "Response body from with sku: {}, fetch_product_by_sku: {:?}",
            sku, products
        );

        // If the list is empty, return an error
        if products.is_empty() {
            return Err(format!("No product found with SKU: {}", sku).into());
        }

        // Return the first product
        let found_product = products.into_iter().next().unwrap();
        if found_product.sku != sku {
            return Err(format!(
                "Product SKU mismatch: expected {}, found {}",
                sku, found_product.sku
            )
            .into());
        }
        Ok(found_product)
    }

    async fn fetch_product_by_id(
        &self,
        id: &str,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
        let full_url = format!("{}/wp-json/wc/v3/products/{}", self.base_url, id);
        println!("fetch_product_by_sku : {}", full_url);
        let res = self
            .woocommerce_client
            .get(&full_url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body = res.text().await?; // Get response as a string
        println!(
            "\x1b[38;5;226mResponse body (bright yellow): {}\x1b[0m",
            body
        );
        let products: WooCommerceProduct = serde_json::from_str(&body)?; // Parse JSON manually
        println!(
            "Response body from with sku: {}, fetch_product_by_sku: {:?}",
            id, products
        );

        if products.id != id {
            return Err(format!(
                "Product SKU mismatch: expected {}, found {}",
                id, products.sku
            )
            .into());
        }
        Ok(products)
    }

    async fn fetch_product_variation_by_sku(
        &self,
        parent_id: &str,
        sku: &str,
    ) -> Result<ProductVariation, Box<dyn std::error::Error>> {
        // /wp-json/wc/v3/products/3420061/variations?sku=my_random_sku
        let full_url = format!(
            "{}/wp-json/wc/v3/products/{}/variations?sku={}",
            self.base_url, parent_id, sku
        );
        println!("fetch_product_variation_by_sku : {}", full_url);
        let res = self
            .woocommerce_client
            .get(&full_url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body = res.text().await?; // Get response as a string
        println!("\x1b[38;5;200mResponse body (pinkish): {}\x1b[0m", body);

        let products: Vec<ProductVariation> = serde_json::from_str(&body)?; // Parse JSON manually
        println!(
            "Response body from with sku: {}, fetch_product_variation_by_sku: {:?}",
            sku, products
        );

        // If the list is empty, return an error
        if products.is_empty() {
            return Err(format!("No product variation found with SKU: {}", sku).into());
        }

        // Return the first product
        let found_product = products.into_iter().next().unwrap();
        if found_product.sku != sku {
            return Err(format!(
                "Product SKU mismatch: expected {}, found {}",
                sku, found_product.sku
            )
            .into());
        }
        Ok(found_product)
    }

    async fn fetch_product_variation_by_id(
        &self,
        parent_id: &str,
        id: &str,
    ) -> Result<ProductVariation, Box<dyn std::error::Error>> {
        // /wp-json/wc/v3/products/3420061/variations?sku=my_random_sku
        let full_url = format!(
            "{}/wp-json/wc/v3/products/{}/variations/{}",
            self.base_url, parent_id, id
        );
        println!("fetch_product_variation_by_sku : {}", full_url);
        let res = self
            .woocommerce_client
            .get(&full_url)
            .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let body = res.text().await?; // Get response as a string
        println!("\x1b[38;5;200mResponse body (pinkish): {}\x1b[0m", body);

        let product: ProductVariation = serde_json::from_str(&body)?; // Parse JSON manually
        println!(
            "Response body from with sku: {}, fetch_product_variation_by_sku: {:?}",
            id, product
        );

        if product.id != id {
            return Err(format!(
                "Product SKU mismatch: expected {}, found {}",
                id, product.sku
            )
            .into());
        }
        Ok(product)
    }

    async fn get_or_fetch_product(
        &self,
        redis_conn: &mut MultiplexedConnection,
        product: &WooCommerceProduct,
        new_product: &bool,
    ) -> Option<WooCommerceProduct> {
        // Try to get the product from Redis
        let sku = product.sku.clone();
        let id: String = product.id.clone();
        if !*new_product {
            if let Ok(Some(json)) = redis_conn
                .hget::<_, _, Option<String>>("products", &sku)
                .await
            {
                if let Ok(product) = serde_json::from_str::<WooCommerceProduct>(&json) {
                    println!("Product found in Redis: {:?}", product);
                    // Check if the SKU matches
                    if product.sku == sku && (*new_product == false && !product.id.is_empty()) {
                        return Some(product); // Found in Redis, return it
                    }
                    println!("SKU mismatch: expected {}, found {}", sku, product.sku);
                } else {
                    println!("Failed to deserialize product from Redis. : {}", json);
                }
            }
        }
        if *new_product {
            println!(
                "Product not found in Redis, fetching from WooCommerce API... sku : {} ",
                sku
            );
            return match self.fetch_product_by_sku(&sku).await {
                Ok(product) => {
                    println!("Product found in WooCommerce: {:?}", product);
                    Some(product) // Found in WooCommerce, return it
                }
                Err(e) => {
                    println!("WooCommerce error: (sku) {:?}", e);
                    None // Product not found or API error
                }
            };
        }
        println!(
            "Product not found in Redis, fetching from WooCommerce API... id : {} ",
            id
        );
        match self.fetch_product_by_id(&id).await {
            Ok(product) => {
                println!("Product found in WooCommerce: {:?}", product);
                Some(product) // Found in WooCommerce, return it
            }
            Err(e) => {
                println!("WooCommerce error: (id) {:?}", e);
                None // Product not found or API error
            }
        }
    }

    async fn get_or_fetch_product_variation(
        &self,
        redis_conn: &mut MultiplexedConnection,
        product: &ProductVariation,
        parent_id: String,
        new_product: &bool,
    ) -> Option<ProductVariation> {
        // Try to get the product from Redis
        let sku = product.sku.clone();
        let id = product.id.clone();
        if !*new_product {
            if let Ok(Some(json)) = redis_conn
                .hget::<_, _, Option<String>>("products", &sku)
                .await
            {
                if let Ok(product) = serde_json::from_str::<ProductVariation>(&json) {
                    println!("Product found in Redis: {:?}", product);
                    // Check if the SKU matches
                    if product.sku == sku && !product.id.is_empty() {
                        return Some(product); // Found in Redis, return it
                    }
                    println!("SKU mismatch: expected {}, found {}", sku, product.sku);
                } else {
                    println!("Failed to deserialize product from Redis. : {}", json);
                }
            }
        }
        println!(
            "Product not found in Redis, fetching from WooCommerce API... sku : {} ",
            sku
        );
        if *new_product {
            println!(
                "Product not found in Redis, fetching from WooCommerce API... sku : {} ",
                sku
            );
            return match self.fetch_product_variation_by_sku(&parent_id, &sku).await {
                Ok(product) => {
                    println!("Product found in WooCommerce: {:?}", product);
                    Some(product) // Found in WooCommerce, return it
                }
                Err(e) => {
                    println!("WooCommerce error(variation) by sku: {:?}", e);
                    None // Product not found or API error
                }
            };
        }
        println!(
            "Product not found in Redis, fetching from WooCommerce API... id : {} ",
            id
        );
        match self.fetch_product_variation_by_id(&parent_id, &id).await {
            Ok(product) => {
                println!("Product found in WooCommerce: {:?}", product);
                Some(product) // Found in WooCommerce, return it
            }
            Err(e) => {
                println!("WooCommerce error(variation) by id: {:?}", e);
                None // Product not found or API error
            }
        }
    }
}

impl WooCommerceProcessor {
    // NEW BATCH CONVENIENCE METHODS
    async fn start_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        products_in_batch: usize,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .start_batch(batch_number, total_batches, products_in_batch)
            .await
    }

    async fn pause_for_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        delay_minutes: u32,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .pause_for_batch(batch_number, total_batches, delay_minutes)
            .await
    }

    async fn complete_batch(
        &self,
        batch_number: usize,
        total_batches: usize,
        successful: usize,
        failed: usize,
    ) -> Result<(), redis::RedisError> {
        self.result
            .read()
            .await
            .complete_batch(batch_number, total_batches, successful, failed)
            .await
    }
}

pub async fn process_woocommerce_csv(file_queue: NewFileProcessQueue) -> Result<(), String> {
    let file_queue = file_queue.clone();
    let base_url = &file_queue.site_details.url;
    let consumer_key = &file_queue.site_details.key;
    let consumer_secret = &file_queue.site_details.secret;
    let file_path = &file_queue.file;
    let file_id = file_path.split('.').next().unwrap_or("").to_string();
    let batch_size = file_queue.batch_size as usize;
    let batch_delay_minutes = file_queue.batch_delay_minutes;
    let dry_run = file_queue.dry_run; // extract dry_run

    if dry_run {
        println!(
            "{}",
            format!("🔍 STARTING DRY RUN MODE - No actual API calls will be made")
                .bright_cyan()
                .bold()
        );
    }

    println!("Processing CSV: {:?}", file_queue);

    // Count total rows first
    let mut rdr = Reader::from_path(get_upload_path(file_path))
        .map_err(|e| format!("Failed to read CSV: {}", e))?;
    let total_row_count: u32 = rdr.records().count().try_into().unwrap();

    let rows_to_process = if file_queue.row_count == 0 {
        total_row_count - file_queue.start_row
    } else {
        file_queue
            .row_count
            .min(total_row_count - file_queue.start_row)
    };
    let rows_to_process = rows_to_process.min(40_000);

    // Create processor with ProcessingResult initialized
    let processor = Arc::new(
        WooCommerceProcessor::new(
            base_url.to_owned(),
            consumer_key.to_owned(),
            consumer_secret.to_owned(),
            file_path.to_string(),
            file_id,
            rows_to_process as usize,
            batch_size,
            batch_delay_minutes,
            dry_run,
        )
        .await
        .map_err(|e| format!("Failed to create processor: {}", e))?,
    );

    //print processor details
    println!("Processor details: {:?}", processor);

    let start = Instant::now();
    let result = processor
        .process_csv(file_path, &file_queue.wordpress_field_mapping, &file_queue)
        .await;

    let duration = start.elapsed();
    println!(
        "{}",
        format!("Total time taken for processing: {:?}", duration)
            .on_purple()
            .yellow()
    );

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // Mark processing as failed
            processor.fail_processing(e.to_string()).await.unwrap_or(());
            Err(format!("Error processing CSV: {}", e))
        }
    }
}
