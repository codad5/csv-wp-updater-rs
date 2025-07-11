use csv::Reader;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::helper::file_helper::get_upload_path;
use crate::helper::{calculate_hash, clean_string};
use crate::libs::redis::FileProcessingManager;
use crate::types::csv_field_woo_mapper::{AttributeMapping, WordPressFieldMapping};
use crate::types::woocommerce::{
    woo_build_product, ProductAttribute, ProductVariation, WooCommerceProduct, WooProduct,
};
use crate::worker::NewFileProcessQueue;

use tokio::sync::Semaphore;



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

impl ProcessingProgress {
    pub async fn increment_failed_rows(
        &mut self,
        file_id: &str,
        redis_conn: &mut redis::aio::Connection,
    ) {
        self.failed_rows += 1;
        self.processed_rows += 1;
        self.sync_to_redis(file_id, redis_conn).await;
    }

    pub async fn sync_to_redis(&self, file_id: &str, redis_conn: &mut redis::aio::Connection) {
        if let Ok(json) = serde_json::to_string(&self) {
            let _: () = redis_conn
                .hset("file_progress", file_id, json)
                .await
                .unwrap_or(());
        }
    }
}
#[derive(Debug, Clone)]
struct WooCommerceProcessor {
    woocommerce_client: Arc<Client>,
    redis_client: redis::Client,
    progress: Arc<Mutex<ProcessingProgress>>,
    base_url: String,
    consumer_key: String,
    consumer_secret: String,
}

impl WooCommerceProcessor {
    async fn new(base_url: String, consumer_key: String, consumer_secret: String) -> Self {
        let woocommerce_client = Client::new();
        let woocommerce_client = match Client::builder()
            .danger_accept_invalid_certs(true) // 👈 Ignore SSL errors
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                println!("Error creating WooCommerce client: {:?}", e);
                woocommerce_client
            }
        };
        let redis_client =
            redis::Client::open("redis://redis:6379/").expect("Failed to create Redis client");
        // base url but trip off any trailing slash
        let base_url = if base_url.ends_with('/') {
            base_url.trim_end_matches('/').to_string()
        } else {
            base_url
        };
        WooCommerceProcessor {
            woocommerce_client: Arc::new(woocommerce_client),
            redis_client,
            progress: Arc::new(Mutex::new(ProcessingProgress::default())),
            base_url,
            consumer_key,
            consumer_secret,
        }
    }

    async fn process_csv(
        self,
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
        let file_id = file_path.split('.').next().unwrap_or("").to_string();
        FileProcessingManager::start_file_process(file_id.as_str(), 10000)
            .await
            .unwrap_or(());

        // First pass to count total rows
        let mut rdr = Reader::from_path(get_upload_path(file_path))?;
        let total_row_count: u32 = rdr.records().count().try_into().unwrap();
        println!("Processing CSV file: {}", file_path);
        println!("Total rows in CSV: {}", total_row_count);
        println!("Processing from row {} for {} rows", start_row, no_of_rows);

        // Reset progress
        let mut progress = self.progress.lock().await;
        *progress = ProcessingProgress::default();

        // let no_of_rows = 39_000;

        // Set total rows to process based on parameters
        let rows_to_process = if no_of_rows == 0 {
            total_row_count - start_row
        } else {
            no_of_rows.min(total_row_count - start_row)
        };

        let max_rows_to_process = 40_000;
        let rows_to_process = rows_to_process.min(max_rows_to_process);

        progress.total_rows = rows_to_process as usize;
        drop(progress);

        // Reset reader
        let mut rdr = Reader::from_path(get_upload_path(file_path))?;
        let headers = rdr.headers()?.clone();
        let reverse_mapping = field_mapping.get_reverse_mapping();
        let reverse_mapping: HashMap<String, String> = reverse_mapping
            .iter()
            .map(|(k, v)| {
                let clean_key = clean_string(k);
                (clean_key, v.clone())
            })
            .collect();
        let reverse_attribute_mapping = field_mapping.get_inverted_attribute();
        let reverse_attribute_mapping: HashMap<String, AttributeMapping> =
            reverse_attribute_mapping
                .iter()
                .map(|(k, v)| {
                    let clean_key = clean_string(k);
                    (clean_key, v.clone())
                })
                .collect();

        println!("Processing records...");

        let new_self = Arc::new(self.clone());
        // Create a semaphore to limit concurrent tasks
        let max_concurrency: usize = (total_row_count / 10).clamp(100, 300).try_into().unwrap();
        println!("\x1b[38;5;166mSpawning with concurrency limit: {max_concurrency}\x1b[0m");
        let semaphore = Arc::new(Semaphore::new(max_concurrency)); // Limit to 40 concurrent tasks
        let redis_client = self.redis_client.clone();
        let progress_clone = Arc::clone(&self.progress);
        let _file_id = Arc::new(file_id.to_string());

        let mut count = 0;

        let record_vec: Vec<Result<csv::StringRecord, csv::Error>> = rdr.records().collect();

        let record_vec = record_vec
            .into_iter()
            .skip(start_row as usize)
            .collect::<Vec<_>>();
        let record_vec = record_vec
            .into_iter()
            .take(rows_to_process as usize)
            .collect::<Vec<_>>();
        let total_row_count = record_vec.len() as u32;

        // i want to make it in this pattern
        // vec![(parent, all_child)]
        // vec![(WooCommerceProduct, Vec<WooCommerceProduct>)]

        let grouped_products = Self::group_products_by_parent(
            record_vec,
            &headers,
            &reverse_mapping,
            &reverse_attribute_mapping,
        )?;
        println!(
            "Number of parents/main products: {}",
            grouped_products.len()
        );
        let (products_with_children, products_without_children): (Vec<_>, Vec<_>) = grouped_products
        .into_iter()
        .partition(|(_, children)| !children.is_empty());

        print!("\x1B[2J\x1B[1;1H"); // Clear the console

        println!(
            "No Products without children (standalone products): {}",
            products_without_children.len()
        );
        println!(
            "Number of standalone products: {}",
            products_without_children.len()
        );

        let standalone_products: Vec<WooCommerceProduct> = products_without_children
        .into_iter()
        .map(|(parent, _)| parent)
        .collect();

        if !standalone_products.is_empty() {
            let standalone_product_len = standalone_products.len();
            println!("Processing {} standalone products in batches", standalone_product_len);
            
            // Define batch size (adjust as needed)
            let batch_size = 50; // or whatever size works best for your API
            let batches: Vec<Vec<WooCommerceProduct>> = standalone_products
                .chunks(batch_size)
                .map(|chunk| chunk.to_vec())
                .collect();
            
            println!("Split into {} batches of up to {} products each", batches.len(), batch_size);
            
            // Create futures for each batch
            let mut batch_futures = Vec::new();
            
            for (batch_index, batch) in batches.into_iter().enumerate() {
                let batch_len = batch.len();
                let self_clone = Arc::clone(&new_self);
                let file_id_clone = file_id.clone();
                let semaphore_clone = Arc::clone(&semaphore);
                
                let batch_task = tokio::spawn(async move {
                    // Acquire semaphore permit for this batch
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    
                    println!("Processing batch {} with {} products", batch_index + 1, batch_len);
                    
                    let batch_tuple = if new_product {
                        (batch, Vec::new()) // (create, update)
                    } else {
                        (Vec::new(), batch) // (create, update)
                    };
                    
                    match self_clone.batch_update_products(batch_tuple).await {
                        Ok(_) => {
                            println!("✅ Batch {} processed successfully ({} products)", batch_index + 1, batch_len);
                            
                            // Update progress for this batch
                            FileProcessingManager::increment_progress(
                                &file_id_clone,
                                batch_len as u32,
                            )
                            .await
                            .unwrap_or_else(|e| println!("Failed to update progress for batch {}: {:?}", batch_index + 1, e));
                            
                            Ok(batch_len)
                        }
                        Err(e) => {
                            println!("❌ Batch {} processing failed: {:?}", batch_index + 1, e);
                            
                            // Still update progress even for failed batches (you might want to handle this differently)
                            FileProcessingManager::increment_progress(
                                &file_id_clone,
                                batch_len as u32,
                            )
                            .await
                            .unwrap_or_else(|e| println!("Failed to update progress for failed batch {}: {:?}", batch_index + 1, e));
                            
                            // Convert error to String to make it Send
                            Err(format!("Batch processing error: {:?}", e))
                        }
                    }
                });
                
                batch_futures.push(batch_task);
            }
            
            // Wait for all batch tasks to complete
            let mut total_processed = 0;
            let mut total_failed = 0;
            
            for (batch_index, batch_task) in batch_futures.into_iter().enumerate() {
                match batch_task.await {
                    Ok(Ok(processed_count)) => {
                        total_processed += processed_count;
                        println!("Batch {} completed: {} products processed", batch_index + 1, processed_count);
                    }
                    Ok(Err(error_msg)) => {
                        // Batch processing failed, but we already logged it above
                        println!("Batch {} failed with error: {}", batch_index + 1, error_msg);
                        total_failed += 1;
                    }
                    Err(join_error) => {
                        println!("Batch {} task failed to complete: {:?}", batch_index + 1, join_error);
                        total_failed += 1;
                    }
                }
            }
            
            println!("🎉 All standalone product batches completed!");
            println!("   Total processed: {}", total_processed);
            println!("   Total failed batches: {}", total_failed);
            
            // Final progress update for standalone products (optional, since we already updated per batch)
            // This ensures the progress bar reflects the completion of all standalone products
            // FileProcessingManager::increment_progress(&file_id, total_processed as u32).await.unwrap_or(());
        }
        let mut parent_futures = Vec::new();
        for (parent, children) in products_with_children {
            println!(
                "\x1b[33mNumber of children for parent {}: {}\x1b[0m",
                parent.sku,
                children.len()
            );
            let redis_client_clone = redis_client.clone();
            let progress_clone = Arc::clone(&progress_clone);
            let semaphore_clone = Arc::clone(&semaphore);
            let new_self_clone = Arc::clone(&new_self);
            let file_id_clone = Arc::clone(&_file_id); // Clone the file_id for each task

            let parent_task = tokio::spawn(async move {
                let start = Instant::now();
                // println!("Processing Parent: {}", parent.sku);
                // print parent in yellow with avaliable permit in purple
                let _permit = semaphore_clone.acquire().await.unwrap();
                let mut redis_conn =
                    match redis_client_clone.get_multiplexed_async_connection().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            println!("Redis connection error: {:?}", e);
                            let mut progress = progress_clone.lock().await;
                            progress.failed_rows += 1;
                            progress.processed_rows += 1;
                            return;
                        }
                    };

                let mut parent_id = parent.id.clone();

                if let Some(prod_id) =
                    Self::compare_product_last_instance(&parent, &mut redis_conn).await
                {
                    parent_id = prod_id;
                } else {
                    match new_self_clone
                        .handle_main_product(&parent, &mut redis_conn, &new_product)
                        .await
                    {
                        Ok(updated_parent) => {
                            println!("Parent processed successfully: {:?}", updated_parent);
                            parent_id = updated_parent.id.clone();
                            let json_body =
                                serde_json::to_string(&updated_parent).unwrap_or("{}".to_string());
                            let _: () = redis_conn
                                .hset("products", &updated_parent.sku, json_body)
                                .await
                                .unwrap_or(());
                            // let newItem =
                            // TODO:for testing something
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
                            FileProcessingManager::increment_progress(
                                file_id_clone.as_str(),
                                total_row_count,
                            )
                            .await
                            .unwrap_or(());
                            let mut progress = progress_clone.lock().await;
                            progress.successful_rows += 1;
                            progress.processed_rows += 1;
                        }
                        Err(e) => {
                            println!("Error processing parent: {:?}", e);
                            FileProcessingManager::increment_progress(
                                file_id_clone.as_str(),
                                total_row_count,
                            )
                            .await
                            .unwrap_or(());
                            let mut progress = progress_clone.lock().await;
                            progress.failed_rows += 1;
                            progress.processed_rows += 1;
                            return;
                        }
                    };
                }

                // Now spawn tasks for the children
                let mut child_futures = Vec::new();
                if parent_id.is_empty() {
                    println!("No parent ID for product {:?}", new_self_clone);
                    return;
                }
                let parent_id = Arc::new(parent_id);
                for child in children {
                    let redis_client_clone = redis_client_clone.clone();
                    let progress_clone = Arc::clone(&progress_clone);
                    let semaphore_clone = Arc::clone(&semaphore_clone); // Clone from the already cloned version
                    let new_self_clone = Arc::clone(&new_self_clone); // Clone from the already cloned version
                    let parent_id_clone = Arc::clone(&parent_id); // Clone the parent_id
                    let file_id_clone = Arc::clone(&file_id_clone); // Clone the file_id for each task
                    let child_task = tokio::spawn(async move {
                        let _permit = semaphore_clone.acquire().await.unwrap();
                        let start = Instant::now();
                        // println!("Processing Child: {} \nAvailable permits: {}", child.sku, semaphore_clone.available_permits());
                        // print child sku and avaliable permit in purple
                        println!("\x1b[35mProcessing Child: {} \n\x1b[0m", child.sku);

                        // let _permit = semaphore_clone.acquire().await.unwrap();
                        println!("permit acquired for child: {}", child.sku);

                        let mut redis_conn =
                            match redis_client_clone.get_multiplexed_async_connection().await {
                                Ok(conn) => conn,
                                Err(e) => {
                                    println!("Redis connection error: {:?}", e);
                                    let mut progress = progress_clone.lock().await;
                                    progress.failed_rows += 1;
                                    progress.processed_rows += 1;
                                    return;
                                }
                            };
                        if let None =
                            Self::compare_product_variation_last_instance(&child, &mut redis_conn)
                                .await
                        {
                            match new_self_clone
                                .handle_variation_product(
                                    &child,
                                    &parent_id_clone,
                                    &mut redis_conn,
                                    &new_product,
                                )
                                .await
                            {
                                Ok(updated_child) => {
                                    println!(
                                        "Child processed successfully: {:?} with parent_id: {:?}",
                                        updated_child, parent_id_clone
                                    );
                                    // parent_id = updated_parent.id.clone();
                                    let json_body = serde_json::to_string(&updated_child)
                                        .unwrap_or("{}".to_string());
                                    let _: () = redis_conn
                                        .hset("products", &updated_child.sku, json_body)
                                        .await
                                        .unwrap_or(());
                                    // TODO:what i added newly
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
                                    FileProcessingManager::increment_progress(
                                        file_id_clone.as_str(),
                                        total_row_count,
                                    )
                                    .await
                                    .unwrap_or(());
                                    let mut progress = progress_clone.lock().await;
                                    progress.successful_rows += 1;
                                    progress.processed_rows += 1;
                                }
                                Err(e) => {
                                    println!("Error processing child: {:?}", e);
                                    FileProcessingManager::increment_progress(
                                        file_id_clone.as_str(),
                                        total_row_count,
                                    )
                                    .await
                                    .unwrap_or(());
                                    let mut progress = progress_clone.lock().await;
                                    progress.failed_rows += 1;
                                    progress.processed_rows += 1;
                                    // return;
                                }
                            };
                        }
                        // print available permits
                        let duration = start.elapsed();
                        println!(
                            "\x1b[97;48;5;42mTime taken for child {}: {:?}\x1b[0m",
                            child.sku, duration
                        );
                    });
                    child_futures.push(child_task);
                }
                // Wait for all children to complete
                for child_task in child_futures {
                    if let Err(e) = child_task.await {
                        println!("Child task error: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                    } else {
                        count += 1;
                    }
                }
                let duration = start.elapsed();
                println!(
                    "\x1b[97;48;5;42mTime taken for parent {}: {:?}\x1b[0m",
                    parent.sku, duration
                );
            });
            parent_futures.push(parent_task);
        }

        // futures::future::join_all(parent_futures).await;
        // Mark as done only if we processed all requested rows successfully
        // if completed_count == processed_count {
        // }

        let mut completed_count = 0;
        for task in parent_futures {
            if let Err(e) = task.await {
                println!("Task error: {:?}", e);
                FileProcessingManager::mark_as_failed(file_id.as_str())
                    .await
                    .unwrap_or(());
            } else {
                completed_count += 1;
            }
        }
        FileProcessingManager::mark_progress(&file_id, 100, 100)
            .await
            .unwrap_or(());
        FileProcessingManager::mark_as_done(&_file_id)
            .await
            .unwrap_or(());

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
    ) -> Result<
        Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // HashMap to store parent SKU/ID -> vector of children
        let mut parent_children_map: std::collections::HashMap<String, Vec<ProductVariation>> =
            std::collections::HashMap::new();

        // Vector to store parent products
        let mut parent_products: Vec<WooCommerceProduct> = Vec::new();

        println!(
            "\x1b[38;5;82mReverse Mapping Debug Info: {:?}\x1b[0m",
            reverse_mapping
        );
        println!(
            "\x1b[38;5;196mAttribute Reverse Mapping Debug Info: {:?}\x1b[0m",
            attribute_reverse
        );

        // Process each record once - O(n) single pass
        for record_result in records {
            let record = match record_result {
                Ok(record) => record,
                Err(e) => {
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
                "\x1b[38;5;45mProduct HashMap Debug Info: {:?}\x1b[0m",
                row_map
            );
            println!(
                "\x1b[38;5;208mProduct HashMap Debug Info: {:?}\x1b[0m",
                attribute_row_map
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
                    println!("Error building product from record");
                    continue;
                }
            }
        }

        // Create the final result structure - O(p) where p is number of parents
        let result: Vec<(WooCommerceProduct, Vec<ProductVariation>)> = parent_products
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

        Ok(result)
    }

    async fn update_product(
        &self,
        product: &WooCommerceProduct,
    ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
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
        // amke id empty
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

    async fn get_progress(&self) -> ProcessingProgress {
        self.progress.lock().await.clone()
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

pub async fn process_woocommerce_csv(file_queue: NewFileProcessQueue) -> Result<(), String> {
    let file_queue = file_queue.clone();
    let base_url = &file_queue.site_details.url;
    let consumer_key = &file_queue.site_details.key;
    let consumer_secret = &file_queue.site_details.secret;

    println!("Processing CSV: {:?}", file_queue);
    println!("Base URL: {}", base_url);
    println!("Consumer Key: {}", consumer_key);

    let processor = WooCommerceProcessor::new(
        base_url.to_owned(),
        consumer_key.to_owned(),
        consumer_secret.to_owned(),
    )
    .await;
    println!("Processor created");
    println!("Processing CSV file...");

    let start = Instant::now();
    let r = match processor
        .process_csv(
            &file_queue.file,
            &file_queue.wordpress_field_mapping,
            &file_queue,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Error processing CSV: {}", e)),
    };

    let duration = start.elapsed();

    // Define the background and text color
    let bg_color = "\x1b[48;5;131m"; // Purple background
    let text_color = "\x1b[38;5;220m"; // Light Yellow text

    // Calculate padding based on terminal width
    let terminal_width = 80; // Adjust this based on your terminal size
    let message = format!("Total time taken for processing: {:?}", duration);

    // Padding calculation
    let padding_size = (terminal_width - message.len()) / 2;
    let padding = " ".repeat(padding_size);

    // Print the padded and styled message
    println!(
        "{}{}{}{}{}",
        padding, bg_color, text_color, message, "\x1b[0m"
    );

    r
}
