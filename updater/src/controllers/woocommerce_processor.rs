use csv::Reader;
use redis::AsyncCommands;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

use crate::helper::file_helper::get_upload_path;
use crate::libs::redis::get_progress;
use crate::types::csv_field_woo_mapper::WordPressFieldMapping;
use crate::worker::{NewFileProcessQueue};

use tokio::sync::Semaphore;


#[derive(Debug, Serialize, Deserialize)]
struct WooCommerceProduct {
  // Core product details
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  sku: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  regular_price: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  sale_price: Option<String>,
  #[serde(skip_serializing_if = "String::is_empty")]
  description: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  short_description: String,

  // Categorization
  #[serde(skip_serializing_if = "Vec::is_empty")]
  categories: Vec<Category>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  tags: Vec<String>,

  // Images
  #[serde(skip_serializing_if = "Vec::is_empty")]
  images: Vec<ProductImage>,

  // Variations support
  #[serde(skip_serializing_if = "Vec::is_empty")]
  variations: Vec<ProductVariation>,

  // Additional attributes
  #[serde(skip_serializing_if = "Vec::is_empty")]
  attributes: Vec<ProductAttribute>,

  // Stock and shipping
  #[serde(skip_serializing_if = "Option::is_none")]
  manage_stock: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  stock_quantity: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  shipping_class: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Category {
  id: Option<i32>,
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductImage {
  #[serde(skip_serializing_if = "String::is_empty")]
  src: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductVariation {
  #[serde(skip_serializing_if = "String::is_empty")]
  sku: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  regular_price: String,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  attributes: Vec<VariationAttribute>,
  #[serde(skip_serializing_if = "Option::is_none")]
  stock_quantity: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductAttribute {
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  visible: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  variation: Option<bool>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VariationAttribute {
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  option: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ProcessingProgress {
  total_rows: usize,
  processed_rows: usize,
  successful_rows: usize,
  failed_rows: usize,
  new_entries: usize,
}

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
    let redis_client =
      redis::Client::open("redis://redis:6379/").expect("Failed to create Redis client");
    WooCommerceProcessor {
      woocommerce_client : Arc::new(woocommerce_client),
      redis_client,
      progress: Arc::new(Mutex::new(ProcessingProgress::default())),
      base_url,
      consumer_key,
      consumer_secret,
    }
  }

  // async fn process_csv(&self, file_path: &str, field_mapping: &WordPressFieldMapping) -> Result<(), Box<dyn std::error::Error>> {
  //   let mut rdr = Reader::from_path(get_upload_path(file_path))?;
  //   let mut redis_conn = self.redis_client.get_multiplexed_async_connection().await?;
  //   println!("Connected to Redis");
  //   println!("Processing CSV file: {}", file_path);
  //   // Reset progress
  //   let mut progress = self.progress.lock().await;
  //   *progress = ProcessingProgress::default();
  //   progress.total_rows = rdr.records().count();
  //   drop(progress);

  //   println!("progress tracking rows");

  //   // Reset reader
  //   let mut rdr = Reader::from_path(get_upload_path(file_path))?;
  //   let headers = rdr.headers()?.clone();

  //   // Group rows by parent SKU for handling variations
  //   // let mut product_groups: HashMap<String, Vec<csv::StringRecord>> = HashMap::new();
  //   let mut product_groups: HashMap<String, HashMap<String, String>> = HashMap::new();
  //   println!("Grouping products by SKU...");
  //   // println!("Total rows: {}", rdr.records().count());
  //   println!("Processing records...");
  //   // println!("All records: {:?}", rdr.records());
  //  let mut iter = rdr.records();
  //  while let Some(result) = iter.next() {
  //       println!("Processing record... counting");
  //       let record = result;
  //       if record.is_err() {
  //           println!("Error reading record: {:?}", record);
  //           continue;
  //       }
  //       let record = record.unwrap();
  //       println!("Processing record: {:?}", record);
  //       let sku = record.get(0).unwrap_or("").to_string();
  //       let record_type = record.get(24).unwrap_or("").to_string();
  //       println!("Record type: {}", record_type);
  //       println!("Record SKU: {}", sku);
  //       let reverse_mapping = field_mapping.get_reverse_mapping();

  //       let row_map: HashMap<String, String> = headers
  //           .iter()
  //           .zip(record.iter())  // Pair headers with row values
  //           .map(|(h, v)| (reverse_mapping.get(h).unwrap_or(&"".to_string()).to_string(), v.to_string()))
  //           .collect();

  //       println!("{:?}", row_map);  // Print each row as a HashMap
  //       product_groups
  //           .entry(sku.clone())
  //           .or_insert_with(HashMap::new)
  //           .extend(row_map);  // Add the row to the product group
        
  //       // if record_type.contains("PRODUCT") {
  //       //     product_groups.entry(sku.clone()).or_default().push(record);
  //       // } else if record_type.contains("MODEL") {
  //       //     let parent_sku = record.get(10).unwrap_or("").to_string();
  //       //     product_groups.entry(parent_sku).or_default().push(record);
  //       // }
  //   }

  //   println!("Total product groups: {}", product_groups.len());
  //   println!(
  //     "Product groups: {:?}",
  //     product_groups
  //   );
  //   println!("Processing products...");


  //   let main_product: HashMap<String, HashMap<String, String>> = product_groups
  //     .into_iter()
  //     .filter(|(_, records)| {
  //         records.get("type").unwrap_or(&"".to_string()) == "PRODUCT"
  //     })
  //     .collect();

  //   // Process each product group
  //   for (parent_sku, records) in main_product {
  //     // // Check Redis for existing SKU
  //     let exists: bool = redis_conn.hexists::<_, _, bool>("products", &parent_sku).await?;

  //     if !exists {
  //         println!("Processing product: {}", parent_sku);
  //         let product = self.woo_product_builder(&records)?;
  //         let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
  //         println!("Product: {:?}", product);
  //         println!("JSON Body: {}", json_body);
          
  //     }

  //     // if !exists {
  //     //   // Prepare product data
  //     //   let product = self.build_product(main_product, &records)?;

  //     //   // Send to WooCommerce
  //     //   let response = self.upload_product(&product).await?;
  //     //   println!("Response: {:?}", response);

  //     //   if response.status().is_success() {
  //     //     // Store in Redis
  //     //     redis_conn
  //     //       .hset("products", &parent_sku, serde_json::to_string(&product)?)
  //     //       .await?;

  //     //     let mut progress = self.progress.lock().await;
  //     //     progress.successful_rows += 1;
  //     //     progress.new_entries += 1;
  //     //   } else {
  //     //     let mut progress = self.progress.lock().await;
  //     //     progress.failed_rows += 1;
  //     //   }
  //     // }

  //     // let mut progress = self.progress.lock().await;
  //     // progress.processed_rows += 1;
  //   }

  //   Ok(())
  // }

async fn process_csv(&self, file_path: &str, field_mapping: &WordPressFieldMapping) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    println!("Processing CSV file: {}", file_path);
    
    // Reset progress
    let mut progress = self.progress.lock().await;
    *progress = ProcessingProgress::default();
    progress.total_rows = rdr.records().count();
    drop(progress);

    // Reset reader
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    let headers = rdr.headers()?.clone();
    let reverse_mapping = field_mapping.get_reverse_mapping();

    println!("Processing records...");
    
    // Create a semaphore to limit concurrent tasks
    let semaphore = Arc::new(Semaphore::new(5)); // Limit to 5 concurrent tasks
    let woo_client = self.woocommerce_client.clone();
    let redis_client = self.redis_client.clone();
    let progress_clone = Arc::clone(&self.progress);
    
    // Create a vector of futures for batch processing
    let mut processing_futures = Vec::new();
    
    let mut iter = rdr.records();
    while let Some(result) = iter.next() {
        let record = match result {
            Ok(rec) => rec,
            Err(e) => {
                println!("Error reading record: {:?}", e);
                let mut progress = self.progress.lock().await;
                progress.failed_rows += 1;
                continue;
            }
        };
        
        let sku = record.get(0).unwrap_or("").to_string();
        let record_type = record.get(24).unwrap_or("").to_string();

        // Only process if it's a PRODUCT (main product)
        if record_type == "PRODUCT" {
            let row_map: HashMap<String, String> = headers
                .iter()
                .zip(record.iter())
                .map(|(h, v)| (reverse_mapping.get(h).unwrap_or(&"".to_string()).to_string(), v.to_string()))
                .collect();
            
            // Clone necessary values for the async task
            let sku_clone = sku.clone();
            let redis_client_clone = redis_client.clone();
            let row_map_clone = row_map.clone();
            let progress_clone = Arc::clone(&progress_clone);
            let woo_client_clone = Arc::clone(&woo_client);
            let semaphore_clone = Arc::clone(&semaphore);
            
            // Spawn a new task for each product
            let task = tokio::spawn(async move {
                // Acquire a permit from the semaphore - this will block if we already have 5 tasks running
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                // Simple approach - fail fast with no retries
                let mut redis_conn = match redis_client_clone.get_multiplexed_async_connection().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        println!("Redis connection error: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                        return;
                    }
                };
                
                // Check Redis for existing SKU
                let exists: bool = match redis_conn.hexists::<_, _, bool>("products", &sku_clone).await {
                    Ok(exists) => exists,
                    Err(e) => {
                        println!("Redis error: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                        return;
                    }
                };
                
                // Skip if SKU already exists
                if exists {
                    let mut progress = progress_clone.lock().await;
                    // progress.skipped_rows += 1;
                    progress.processed_rows += 1;
                    return;
                }
                
                // Create product
                let product = match Self::woo_product_builder(&row_map_clone) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Error building product: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                        return;
                    }
                };
                
                let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
                
                // Upload to WooCommerce
                let response = woo_client_clone.post(&format!("{}/wp-json/wc/v3/products", "your_base_url"))
                    .basic_auth("your_consumer_key", Some("your_consumer_secret"))
                    .body(json_body.clone())
                    .send()
                    .await;
                    
                match response {
                    Ok(res) => {
                        if res.status().is_success() {
                            // Update Redis cache
                            if let Err(e) = redis_conn.hset::<_, _, _, ()>("products", &sku_clone, json_body).await {
                                println!("Failed to update Redis cache: {:?}", e);
                            }
                            
                            let mut progress = progress_clone.lock().await;
                            progress.successful_rows += 1;
                            progress.new_entries += 1;
                            progress.processed_rows += 1;
                        } else {
                            // API error
                            println!("WooCommerce API error: {:?}", res.status());
                            let mut progress = progress_clone.lock().await;
                            progress.failed_rows += 1;
                            progress.processed_rows += 1;
                        }
                    },
                    Err(e) => {
                        println!("WooCommerce request error: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                    }
                }
                
                // The permit is automatically dropped when this task finishes
            });
            
            processing_futures.push(task);
        }
    }
    
    // Wait for all tasks to complete
    for task in processing_futures {
        if let Err(e) = task.await {
            println!("Task error: {:?}", e);
        }
    }

    println!("CSV processing completed");
    Ok(())
}

  fn woo_product_builder(
    product: &HashMap<String, String>,
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
      
      let get_value = |key: &str| -> String {
          product.get(key).unwrap_or(&"".to_string()).trim().to_string()
      };

      let sku = get_value("sku");
      let title = get_value("name");
      let description = get_value("description");
      let short_description = get_value("short_description");
      let regular_price = get_value("regular_price");
      let sale_price = get_value("sale_price");
      
      // Handle categories
      let categories: Vec<Category> = get_value("category_ids")
          .split('/')
          .filter(|c| !c.trim().is_empty())
          .map(|cat| Category {
              id: None,
              name: cat.trim().to_string(),
              slug: cat.trim().to_lowercase().replace(' ', "-"),
          })
          .collect();

      // Handle images
      let featured_image = get_value("images");
      let gallery_images: Vec<_> = if !featured_image.is_empty() {
          featured_image.split('|')
              .filter(|img| !img.trim().is_empty())
              .collect()
      } else {
          vec![]
      };
      
      let mut images = vec![];
      
      if !featured_image.is_empty() {
          images.push(ProductImage {
              src: featured_image.clone(),
              name: if title.is_empty() { None } else { Some(title.clone()) },
              alt: if title.is_empty() { None } else { Some(title.clone()) },
          });
      }
      
      images.extend(gallery_images.iter().map(|img| ProductImage {
          src: img.trim().to_string(),
          name: if title.is_empty() { None } else { Some(title.clone()) },
          alt: if title.is_empty() { None } else { Some(title.clone()) },
      }));
      
      // Handle attributes
      let material = get_value("material");
      let brand = get_value("brand");
      let mut attributes = vec![];
      
      if !material.is_empty() {
          attributes.push(ProductAttribute {
              name: "Material".to_string(),
              position: Some(1),
              visible: Some(true),
              variation: Some(false),
              options: material.split(',')
                  .map(|m| m.trim().to_string())
                  .filter(|m| !m.is_empty())
                  .collect(),
          });
      }
      
      if !brand.is_empty() {
          attributes.push(ProductAttribute {
              name: "Brand".to_string(),
              position: Some(2),
              visible: Some(true),
              variation: Some(false),
              options: vec![brand.to_string()],
          });
      }

      // Handle stock quantity
      let stock_quantity = get_value("stock_quantity").parse().ok();
      
      // Only include sale_price if it's not empty
      let sale_price_option = if sale_price.is_empty() { 
          None 
      } else { 
          Some(sale_price) 
      };
      
      // Only include shipping_class if it's not empty
      let shipping_class = get_value("shipping_class_id");
      let shipping_class_option = if shipping_class.is_empty() {
          None
      } else {
          Some(shipping_class)
      };
      
      Ok(WooCommerceProduct {
          name: title,
          sku,
          regular_price,
          sale_price: sale_price_option,
          description,
          short_description,
          categories,
          tags: vec![],
          images,
          variations: vec![], // You can implement variations if needed
          attributes,
          manage_stock: if stock_quantity.is_some() { Some(true) } else { None },
          stock_quantity,
          shipping_class: shipping_class_option,
      })
  }



  
  async fn upload_product(
    &self,
    product: &WooCommerceProduct,
  ) -> Result<reqwest::Response, reqwest::Error> {
    let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
    self
      .woocommerce_client
      .post(&format!("{}/wp-json/wc/v3/products", self.base_url))
      .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
      .body(json_body)
      .send()
      .await
  }

  async fn get_progress(&self) -> ProcessingProgress {
    self.progress.lock().await.clone()
  }
}


pub async fn process_woocommerce_csv(
  file_queue: NewFileProcessQueue,
  base_url: String,
  consumer_key: String,
  consumer_secret: String,
) -> Result<(), String> {
  println!("Processing CSV: {:?}", file_queue);
  println!("Base URL: {}", base_url);
  println!("Consumer Key: {}", consumer_key);
  let processor = WooCommerceProcessor::new(base_url, consumer_key, consumer_secret).await;
  println!("Processor created");
  println!("Processing CSV file...");
  match processor.process_csv(&file_queue.file, &file_queue.wordpress_field_mapping).await {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Error processing CSV: {}", e)),
  }
}


/**
 * use tokio::sync::Semaphore;
use std::sync::Arc;

async fn process_csv(&self, file_path: &str, field_mapping: &WordPressFieldMapping) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    println!("Processing CSV file: {}", file_path);
    
    // Reset progress
    let mut progress = self.progress.lock().await;
    *progress = ProcessingProgress::default();
    progress.total_rows = rdr.records().count();
    drop(progress);

    // Reset reader
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    let headers = rdr.headers()?.clone();
    let reverse_mapping = field_mapping.get_reverse_mapping();

    println!("Processing records...");
    
    // Create a semaphore to limit concurrent tasks
    let semaphore = Arc::new(Semaphore::new(5)); // Limit to 5 concurrent tasks
    let woo_client = self.woocommerce_client.clone();
    let redis_client = self.redis_client.clone();
    let progress_clone = Arc::clone(&self.progress);
    
    // Create a vector of futures for batch processing
    let mut processing_futures = Vec::new();
    
    let mut iter = rdr.records();
    while let Some(result) = iter.next() {
        let record = match result {
            Ok(rec) => rec,
            Err(e) => {
                println!("Error reading record: {:?}", e);
                let mut progress = self.progress.lock().await;
                progress.failed_rows += 1;
                continue;
            }
        };
        
        let sku = record.get(0).unwrap_or("").to_string();
        let record_type = record.get(24).unwrap_or("").to_string();

        // Only process if it's a PRODUCT (main product)
        if record_type == "PRODUCT" {
            let row_map: HashMap<String, String> = headers
                .iter()
                .zip(record.iter())
                .map(|(h, v)| (reverse_mapping.get(h).unwrap_or(&"".to_string()).to_string(), v.to_string()))
                .collect();
            
            // Clone necessary values for the async task
            let sku_clone = sku.clone();
            let redis_client_clone = redis_client.clone();
            let row_map_clone = row_map.clone();
            let progress_clone = Arc::clone(&progress_clone);
            let woo_client_clone = Arc::clone(&woo_client);
            let semaphore_clone = Arc::clone(&semaphore);
            
            // Spawn a new task for each product
            let task = tokio::spawn(async move {
                // Acquire a permit from the semaphore - this will block if we already have 5 tasks running
                let _permit = semaphore_clone.acquire().await.unwrap();
                
                // Add exponential backoff for error handling
                let mut retry_count = 0;
                let max_retries = 3;
                
                while retry_count < max_retries {
                    let mut redis_conn = match redis_client_clone.get_multiplexed_async_connection().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            println!("Redis connection error: {:?}", e);
                            retry_count += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(500 * 2_u64.pow(retry_count))).await;
                            continue;
                        }
                    };
                    
                    // Check Redis for existing SKU
                    let exists: bool = match redis_conn.hexists::<_, _, bool>("products", &sku_clone).await {
                        Ok(exists) => exists,
                        Err(e) => {
                            println!("Redis error: {:?}", e);
                            retry_count += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(500 * 2_u64.pow(retry_count))).await;
                            continue;
                        }
                    };
                    
                    if !exists {
                        // Create product
                        let product = match Self::woo_product_builder(&row_map_clone) {
                            Ok(p) => p,
                            Err(e) => {
                                println!("Error building product: {:?}", e);
                                let mut progress = progress_clone.lock().await;
                                progress.failed_rows += 1;
                                break;
                            }
                        };
                        
                        let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
                        
                        // Upload to WooCommerce
                        let response = woo_client_clone.post(&format!("{}/wp-json/wc/v3/products", "your_base_url"))
                            .basic_auth("your_consumer_key", Some("your_consumer_secret"))
                            .body(json_body.clone())
                            .send()
                            .await;
                            
                        match response {
                            Ok(res) => {
                                if res.status().is_success() {
                                    // Update Redis cache
                                    let _: () = redis_conn.hset("products", &sku_clone, json_body).await.unwrap_or(());
                                    
                                    let mut progress = progress_clone.lock().await;
                                    progress.successful_rows += 1;
                                    progress.new_entries += 1;
                                    progress.processed_rows += 1;
                                    
                                    break; // Success, exit the retry loop
                                } else if res.status().as_u16() == 429 {
                                    // Rate limit hit - wait longer
                                    println!("Rate limit hit, backing off...");
                                    retry_count += 1;
                                    tokio::time::sleep(tokio::time::Duration::from_secs(5 * 2_u64.pow(retry_count))).await;
                                    continue;
                                } else {
                                    // Other error
                                    println!("WooCommerce API error: {:?}", res.status());
                                    let mut progress = progress_clone.lock().await;
                                    progress.failed_rows += 1;
                                    progress.processed_rows += 1;
                                    break;
                                }
                            },
                            Err(e) => {
                                println!("WooCommerce request error: {:?}", e);
                                retry_count += 1;
                                if retry_count < max_retries {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * 2_u64.pow(retry_count))).await;
                                    continue;
                                } else {
                                    let mut progress = progress_clone.lock().await;
                                    progress.failed_rows += 1;
                                    progress.processed_rows += 1;
                                    break;
                                }
                            }
                        }
                    } else {
                        // SKU already exists
                        let mut progress = progress_clone.lock().await;
                        progress.skipped_rows += 1;
                        progress.processed_rows += 1;
                        break;
                    }
                }
                
                // The permit is automatically dropped when this task finishes
            });
            
            processing_futures.push(task);
        }
    }
    
    // Wait for all tasks to complete
    for task in processing_futures {
        if let Err(e) = task.await {
            println!("Task error: {:?}", e);
        }
    }

    println!("CSV processing completed");
    Ok(())
}
 */
fn something() {
    
}