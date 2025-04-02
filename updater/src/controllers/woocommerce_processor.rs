use csv::Reader;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use serde::{Serializer, Deserializer};
use std::fmt::Display;
use std::str::FromStr;

use crate::helper::clean_string;
use crate::helper::file_helper::get_upload_path;
use crate::libs::redis::{get_progress, FileProcessingManager};
use crate::types::csv_field_woo_mapper::WordPressFieldMapping;
use crate::worker::{NewFileProcessQueue};

use tokio::sync::Semaphore;


#[derive(Debug, Serialize, Deserialize, Clone)]
struct WooCommerceProduct {
    // Core product details
    #[serde(
        skip_serializing_if = "String::is_empty",
        serialize_with = "serialize_id_as_number",
        deserialize_with = "deserialize_id_as_string",
        default
    )]
    id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    sku: String,
    #[serde(rename = "type", skip_serializing_if = "String::is_empty", default)]
    type_: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    parent: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    regular_price: String,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        default,
        deserialize_with = "deserialize_optional_string"
    )]
    sale_price: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    description: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    short_description: String,
    
    // Categorization
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    categories: Vec<Category>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tags: Vec<String>,
    
    // Images
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    images: Vec<ProductImage>,
    
    // Variations support
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    variations: Vec<ProductVariation>,
    
    // Additional attributes
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    attributes: Vec<ProductAttribute>,
    
    // Stock and shipping
    #[serde(
        skip_serializing_if = "Option::is_none", 
        default,
        deserialize_with = "deserialize_optional_bool_none_if_false"
    )]
    manage_stock: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    stock_quantity: Option<i32>,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        default,
        deserialize_with = "deserialize_optional_string"
    )]
    shipping_class: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Category {
  id: Option<i32>,
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  slug: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct ProductImage {
  #[serde(skip_serializing_if = "String::is_empty")]
  src: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct VariationAttribute {
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  option: String,
}


impl WooCommerceProduct {
    /// Merges two WooCommerceProduct instances, with values from `other` taking precedence
    /// over values from `self` when both exist and are not empty.
    /// 
    /// Returns a new WooCommerceProduct instance with the merged values.
    pub fn merge(&self, other: &WooCommerceProduct) -> WooCommerceProduct {
        // Helper function to merge Vec collections
        fn merge_vec<T: Clone>(a: &[T], b: &[T]) -> Vec<T> {
            if !b.is_empty() {
                b.to_vec()
            } else {
                a.to_vec()
            }
        }

        // Helper function to merge Option values
        fn merge_option<T: Clone>(a: &Option<T>, b: &Option<T>) -> Option<T> {
            if b.is_some() {
                b.clone()
            } else {
                a.clone()
            }
        }

        // Helper function to merge String values
        fn merge_string(a: &str, b: &str) -> String {
            if !b.is_empty() {
                b.to_string()
            } else {
                a.to_string()
            }
        }

        let mut type_ = merge_string(&self.type_, &other.type_);
        // check if type is in array of simple, grouped, external and variable
        if !["simple", "grouped", "external", "variable"].contains(&type_.as_str()) {
            type_  =  String::new(); // set to empty string if not valid
             // Return self if type is not valid
        } 

        WooCommerceProduct {
            // Core product details
            name: merge_string(&self.name, &other.name),
            id: merge_string(&self.id, &other.id),
            sku: merge_string(&self.sku, &other.sku),
            type_,
            regular_price: merge_string(&self.regular_price, &other.regular_price),
            sale_price: merge_option(&self.sale_price, &other.sale_price),
            description: merge_string(&self.description, &other.description),
            short_description: merge_string(&self.short_description, &other.short_description),
            parent: merge_string(&self.parent, &other.parent),
            // Categorization
            categories: merge_vec(&self.categories, &other.categories),
            tags: merge_vec(&self.tags, &other.tags),
            
            // Images
            images: merge_vec(&self.images, &other.images),
            
            // Variations support
            variations: merge_vec(&self.variations, &other.variations),
            
            // Additional attributes
            attributes: merge_vec(&self.attributes, &other.attributes),
            
            // Stock and shipping
            manage_stock: merge_option(&self.manage_stock, &other.manage_stock),
            stock_quantity: merge_option(&self.stock_quantity, &other.stock_quantity),
            shipping_class: merge_option(&self.shipping_class, &other.shipping_class),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Product name is required".to_string());
        }
        if self.sku.is_empty() {
            return Err("Product SKU is required".to_string());
        }
        if self.regular_price.is_empty() {
            return Err("Product regular price is required".to_string());
        }
        if self.description.is_empty() {
            return Err("Product description is required".to_string());
        }
        Ok(())
    }

    // to check if the product has changed or not
    pub fn get_changes(&self, other: &WooCommerceProduct) -> Vec<String> {
        let mut changes = Vec::new();
        
        if self.name != other.name {
            changes.push("name".to_string());
        }
        
        if self.sku != other.sku {
            changes.push("sku".to_string());
        }
        
        if self.description != other.description {
            changes.push("description".to_string());
        }
        
        if self.short_description != other.short_description {
            changes.push("short_description".to_string());
        }
        
        if self.categories != other.categories && !other.categories.is_empty() {
            changes.push("categories".to_string());
        }
        
        if self.tags != other.tags && !other.tags.is_empty() {
            changes.push("tags".to_string());
        }
        
        // if self.images != other.images && !other.images.is_empty() {
        //     changes.push("images".to_string());
        // }
        
        if self.variations != other.variations && !other.variations.is_empty() {
            changes.push("variations".to_string());
        }
        
        if self.attributes != other.attributes && !other.attributes.is_empty() {
            changes.push("attributes".to_string());
        }
        
        if self.manage_stock != other.manage_stock {
            changes.push("manage_stock".to_string());
        }
        
        if self.stock_quantity != other.stock_quantity {
            changes.push("stock_quantity".to_string());
        }
        
        if self.shipping_class != other.shipping_class {
            changes.push("shipping_class".to_string());
        }
        
        // Only check prices if other.type_ is empty or "simple"
        if other.type_.is_empty() || other.type_ == "simple" {
            if self.regular_price != other.regular_price {
                changes.push("regular_price".to_string());
            }
            
            if self.sale_price != other.sale_price {
                changes.push("sale_price".to_string());
            }
        }
        
        println!("Product with SKU {} has {} changes: {:?}", 
                self.sku, changes.len(), changes);
        
        changes
    }

    pub fn has_changed(&self, other: &WooCommerceProduct) -> bool {
        !self.get_changes(other).is_empty()
    }

    // a method to check if main product or a variation 
    pub fn is_main_product(&self) -> bool {
        self.parent.is_empty()
    }
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
  progress: Arc<Mutex<ProcessingProgress>>,
  base_url: String,
  consumer_key: String,
  consumer_secret: String,
}

impl WooCommerceProcessor {
  async fn new(base_url: String, consumer_key: String, consumer_secret: String) -> Self {
    let woocommerce_client = Client::new();
    let woocommerce_client = match Client::builder()
            .danger_accept_invalid_certs(true)  // 👈 Ignore SSL errors
            .build() {
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
      woocommerce_client : Arc::new(woocommerce_client),
      redis_client,
      progress: Arc::new(Mutex::new(ProcessingProgress::default())),
      base_url,
      consumer_key,
      consumer_secret,
    }
  }

async fn process_csv(self, file_path: &str, field_mapping: &WordPressFieldMapping, start_row: u32, no_of_rows: u32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // File id is file path without ext
    let file_id = file_path.split('.').next().unwrap_or("");
    FileProcessingManager::start_file_process(file_id, 10000).await.unwrap_or(());
    
    // First pass to count total rows
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    let total_row_count: u32 = rdr.records().count().try_into().unwrap();
    println!("Processing CSV file: {}", file_path);
    println!("Total rows in CSV: {}", total_row_count);
    println!("Processing from row {} for {} rows", start_row, no_of_rows);
    
    // Reset progress
    let mut progress = self.progress.lock().await;
    *progress = ProcessingProgress::default();
    
    // Set total rows to process based on parameters
    let rows_to_process = if no_of_rows == 0 {
        total_row_count - start_row
    } else {
        no_of_rows.min(total_row_count - start_row)
    };
    
    progress.total_rows = rows_to_process as usize;
    drop(progress);

    // Reset reader
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    let headers = rdr.headers()?.clone();
    let reverse_mapping = field_mapping.get_reverse_mapping();
    let reverse_mapping: HashMap<String, String> = reverse_mapping.iter()
        .map(|(k, v)| {
            let clean_key = clean_string(k);
            (clean_key, v.clone())
        })
        .collect();

    println!("Processing records...");
    
    let new_self = Arc::new(self.clone());
    // Create a semaphore to limit concurrent tasks
    let semaphore = Arc::new(Semaphore::new(40)); // Limit to 40 concurrent tasks
    let redis_client = self.redis_client.clone();
    let progress_clone = Arc::clone(&self.progress);
  
    // Create a vector of futures for batch processing
    let mut processing_futures = Vec::new();
    
    let mut count = 0;
    let mut processed_count = 0;
    let mut skus: Vec<String> = vec![];
    
    // Skip records before start_row
    let mut iter = rdr.records();
    for _ in 0..start_row {
        if iter.next().is_none() {
            // We've reached the end of file before start_row
            break;
        }
        count += 1;
    }
    
    // Process only up to the specified number of rows
    while let Some(result) = iter.next() {
        if no_of_rows > 0 && processed_count >= no_of_rows {
            break;
        }
        
        let new_self = Arc::clone(&new_self);
        println!("Processing record {}", count + 1);
        
        let record = match result {
            Ok(rec) => rec,
            Err(e) => {
                println!("Error reading record: {:?}", e);
                count += 1;
                processed_count += 1;
                continue;
            }
        };
        
        let sku = record.get(0).unwrap_or("").to_string();
        // Check for duplicate SKU
        if skus.contains(&sku) {
            println!("SKU {} already processed, skipping...", sku);
            count += 1;
            continue;
        } else {
            skus.push(sku.clone());
        }
        
        let record_type = record.get(24).unwrap_or("").to_string();
        println!("Processing record : SKU={}, Type={}", sku, record_type);

        // Map row fields
        let row_map: HashMap<String, String> = headers
            .iter()
            .zip(record.iter())
            .map(|(h, v)| (reverse_mapping.get(h).unwrap_or(&"".to_string()).to_lowercase(), v.to_string()))
            .collect();

        println!("Row map: {:?}", row_map);

        // Clone necessary values for the async task
        let redis_client_clone = redis_client.clone();
        let row_map_clone = row_map.clone();
        let progress_clone = Arc::clone(&progress_clone);
        let semaphore_clone = Arc::clone(&semaphore);
        let sku_clone = sku.clone();
        
        // Spawn a new task for each product
        let task = tokio::spawn(async move {
            println!("Processing product in new async way: {}", new_self.consumer_key);
            // Acquire a permit from the semaphore
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            // Establish Redis connection
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
            
            println!("Redis connection established for SKU: {}", sku_clone);
            
            // Create product
            let mut new_product_update = match Self::woo_product_builder(&row_map_clone) {
                Ok(p) => p,
                Err(e) => {
                    println!("Error building product: {:?}", e);
                    let mut progress = progress_clone.lock().await;
                    progress.failed_rows += 1;
                    progress.processed_rows += 1;
                    return;
                }
            };

            println!("Product created from CSV on row {}: {:?}", count, new_product_update);

            if new_product_update.is_main_product() {
                println!("Handling main product for SKU: {}", new_product_update.sku);
                // Handle main product
                match new_self.handle_main_product(&new_product_update, &mut redis_conn).await {
                    Ok(p) => {
                        println!("Main product handled successfully: {:?}", p);
                        new_product_update = p;
                    },
                    Err(e) => {
                        println!("Error handling main product: {:?}", e);
                        let mut progress = progress_clone.lock().await;
                        progress.failed_rows += 1;
                        progress.processed_rows += 1;
                        return;
                    }
                }
            } else {
                println!("Handling variation for SKU: {}", new_product_update.sku);
                return; //TODO: remove this return
            }

            // Cache the product in redis
            let json_body = serde_json::to_string(&new_product_update).unwrap_or("{}".to_string());
            let _: () = redis_conn.hset("products", &new_product_update.sku, json_body).await.unwrap_or(());
            println!("Product with SKU {} cached in Redis", new_product_update.sku);
            
            // Update progress
            let mut progress = progress_clone.lock().await;
            progress.processed_rows += 1;
            progress.successful_rows += 1;
        });
        
        processing_futures.push(task);
        count += 1;
        processed_count += 1;
    }
    
    println!("Total records selected for processing: {}", processed_count);
    
    // Wait for all tasks to complete and update progress
    let mut completed_count = 0;
    for task in processing_futures {
        if let Err(e) = task.await {
            println!("Task error: {:?}", e);
            FileProcessingManager::mark_as_failed(file_id).await.unwrap_or(());
        } else {
            completed_count += 1;
            FileProcessingManager::mark_progress(file_id, completed_count, rows_to_process)
                .await.unwrap_or(());
        }
    }

    println!("CSV processing completed for the specified rows");
    
    // Mark as done only if we processed all requested rows successfully
    if completed_count == processed_count {
        FileProcessingManager::mark_as_done(file_id).await.unwrap_or(());
    }
    
    Ok(())
}

async fn handle_main_product(&self, product: &WooCommerceProduct, redis_conn: &mut MultiplexedConnection) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
    // if its parent id is empty then its a main product
    if !product.parent.is_empty() {
        return Err("Product is not a main product".into());
    }
    
    let exists = self.get_or_fetch_product(redis_conn, &product.sku).await;
    let mut new_product_update = product.clone();
    println!("new product update: {:?}", new_product_update);
    
    if let Some(found_product) = exists {
        // merge the new product update with the existing
        // check if there is any difference between the merged and the new product update, if any diff call the update method to update through the api
        // if no change skip
        
        // Check core fields that would require an update
        if found_product.has_changed(&new_product_update) {
            
            let update_prod = found_product.merge(&new_product_update);
            // print before and after the merge
            println!("Product before merge: {:?} \nProduct after merge: {:?}", found_product, update_prod);
            let update_prod = self.update_product(&update_prod).await;
            match update_prod {
                Ok(p) => {
                    println!("Product updated successfully: {:?}", p); 
                    new_product_update = p;
                    // let mut progress = progress_clone.lock().await;
                    // progress.successful_rows += 1;
                    // progress.processed_rows += 1;
                },
                Err(e) => {
                    return Err(format!("Error updating product: {:?}", e).into());
                    // let mut progress = progress_clone.lock().await;
                    // progress.failed_rows += 1;
                    // progress.processed_rows += 1;
                }
            }
        }
        else {
            println!("No changes detected for product: {:?}", new_product_update.sku);
        }
    } else {
        // if not found create a new product but an ID must exist
        
        // Ensure required fields are present
        if let Err(e) = new_product_update.validate() {
            let error_msg = format!("Missing required fields for product creation: {:?} in {:?}", e, new_product_update);
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
            },
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


async fn handle_variation_product(&self, product: &WooCommerceProduct, redis_conn: &mut MultiplexedConnection) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
    // if its parent id is empty then its a main product
    if product.parent.is_empty() {
        return Err("Product is a main product".into());
    }
    
    let exists = self.get_or_fetch_product(redis_conn, &product.sku).await;
    let mut new_product_update = product.clone();
    
    if let Some(product) = exists {
        // merge the new product update with the existing
        // check if there is any difference between the merged and the new product update, if any diff call the update method to update through the api
        // if no change skip
        
        // Check core fields that would require an update
        if product.name != new_product_update.name || 
            product.description != new_product_update.description ||
            product.short_description != new_product_update.short_description ||
            product.regular_price != new_product_update.regular_price {
            
            let update_prod = product.merge(&new_product_update);
            let update_prod = self.update_product(&update_prod).await;
            match update_prod {
                Ok(p) => {
                    new_product_update = p;
                    // let mut progress = progress_clone.lock().await;
                    // progress.successful_rows += 1;
                    // progress.processed_rows += 1;
                },
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
            let error_msg = format!("Missing required fields for product creation: {:?} in {:?}", e, new_product_update);
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
            },
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

 fn woo_product_builder(
    product: &HashMap<String, String>,
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
      
      let get_value = |key: &str| -> String {
          product.get(key).unwrap_or(&"".to_string()).trim().to_string()
      };

      let id = get_value("id");
      let sku = get_value("sku");
      let type_ = get_value("type");
      let name = get_value("name");
      let description = get_value("description");
      let short_description = get_value("short_description");
      let regular_price = get_value("regular_price");
      let sale_price = get_value("sale_price");
      let parent = get_value("parent_id");
      
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
              name: if name.is_empty() { None } else { Some(name.clone()) },
              alt: if name.is_empty() { None } else { Some(name.clone()) },
          });
      }
      
      images.extend(gallery_images.iter().map(|img| ProductImage {
          src: img.trim().to_string(),
          name: if name.is_empty() { None } else { Some(name.clone()) },
          alt: if name.is_empty() { None } else { Some(name.clone()) },
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
          id,
          name,
          sku,
          type_,
          parent,
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



  
  async fn update_product(
    &self,
    product: &WooCommerceProduct,
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
    let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
    println!("Updating product with sku: {} and JSON body: {}", product.sku, json_body);
    let res = self
      .woocommerce_client
      .put(&format!("{}/wp-json/wc/v3/products/{}", self.base_url, product.id))
      .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
      .body(json_body)
      .header("Content-Type", "application/json")
      .send()
      .await?;
    let body = res.text().await?; // Get response as a string
    println!("Response body with sku: {}, update_product: {}", product.sku, body);
    let products: WooCommerceProduct = serde_json::from_str(&body)?; // Parse JSON manually

    Ok(products)
  }

  async fn create_product(
    &self,
    product: &WooCommerceProduct,
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
    // amke id empty
    let json_body = serde_json::to_string(&product).unwrap_or("{}".to_string());
    println!("Creating product with JSON body: {} for product id {} and name {}", json_body, product.id, product.name);
    let res = self
      .woocommerce_client
      .post(&format!("{}/wp-json/wc/v3/products", self.base_url))
      .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
      .body(json_body)
      .header("Content-Type", "application/json")
      .send()
      .await?;
    let body = res.text().await?; // Get response as a string
    println!("Response body from create_product: {} for product id {} and name {}", body, product.id, product.name);
    let products: WooCommerceProduct = serde_json::from_str(&body)?; // Parse JSON manually
    

    Ok(products)
  }

  async fn fetch_product_by_sku(
      &self,
      sku: &str,
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
    let full_url = format!("{}/wp-json/wc/v3/products?sku={}", self.base_url, sku);
    println!("fetch_product_by_sku : {}",full_url);
      let res = self
          .woocommerce_client
          .get(&full_url)
          .basic_auth(&self.consumer_key, Some(&self.consumer_secret))
          .header("Content-Type", "application/json")
          .send()
          .await?;

      let body = res.text().await?; // Get response as a string
      
      let products: Vec<WooCommerceProduct> = serde_json::from_str(&body)?; // Parse JSON manually
      println!("Response body from with sku: {}, fetch_product_by_sku: {:?}", sku , products);

      // If the list is empty, return an error
      if products.is_empty() {
          return Err(format!("No product found with SKU: {}", sku).into());
      }

      // Return the first product
      let found_product = products.into_iter().next().unwrap();
      if found_product.sku != sku {
        return Err(format!("Product SKU mismatch: expected {}, found {}", sku, found_product.sku).into());
      }
      Ok(found_product)
  }


  async fn get_progress(&self) -> ProcessingProgress {
    self.progress.lock().await.clone()
  }

  async fn get_or_fetch_product(
    &self,
    redis_conn: &mut MultiplexedConnection,
    sku: &str,
) -> Option<WooCommerceProduct> {
    // Try to get the product from Redis
    if let Ok(Some(json)) = redis_conn.hget::<_, _, Option<String>>("products", sku).await {
        if let Ok(product) = serde_json::from_str::<WooCommerceProduct>(&json) {
            println!("Product found in Redis: {:?}", product);
            // Check if the SKU matches
            if product.sku == sku {
                return Some(product); // Found in Redis, return it
            }
            println!("SKU mismatch: expected {}, found {}", sku, product.sku);
        } else {
            println!("Failed to deserialize product from Redis. : {}", json);
        }
    }
        println!("Product not found in Redis, fetching from WooCommerce API... sku : {} ", sku);

        // If not found in Redis, fetch from WooCommerce API
        match self.fetch_product_by_sku(sku).await {
            Ok(product) => {
                println!("Product found in WooCommerce: {:?}", product);
                Some(product) // Found in WooCommerce, return it
            }
            Err(e) => {
                println!("WooCommerce error: {:?}", e);
                None // Product not found or API error
            }
        }
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
  match processor.process_csv(&file_queue.file, &file_queue.wordpress_field_mapping, file_queue.start_row, file_queue.row_count).await {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Error processing CSV: {}", e)),
  }
}



// Function to serialize ID as a number
pub fn serialize_id_as_number<S, T>(id: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: AsRef<str> + Display,
{
    match id.as_ref().parse::<i64>() {
        Ok(num) => serializer.serialize_i64(num),
        Err(_) => serializer.serialize_str(id.as_ref()),
    }
}

// Function to deserialize ID as a string
pub fn deserialize_id_as_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + serde::Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    use serde::de::Error;
    
    // First try as a string
    let value = serde_json::Value::deserialize(deserializer)?;
    
    match value {
        serde_json::Value::String(s) => {
            T::from_str(&s).map_err(|e| D::Error::custom(format!("Failed to parse string: {}", e)))
        },
        serde_json::Value::Number(n) => {
            let num_str = n.to_string();
            T::from_str(&num_str).map_err(|e| D::Error::custom(format!("Failed to parse number: {}", e)))
        },
        _ => Err(D::Error::custom("Expected string or number")),
    }
}
// Deserialize Option<String> as None if the string is empty
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    
    // Convert Some("") to None
    match opt {
        Some(s) if s.is_empty() => Ok(None),
        _ => Ok(opt),
    }
}

// Deserialize Option<bool> as None if value is Some(false)
fn deserialize_optional_bool_none_if_false<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<bool>::deserialize(deserializer)?;
    
    // Convert Some(false) to None
    match opt {
        Some(false) => Ok(None),
        _ => Ok(opt),
    }
}
