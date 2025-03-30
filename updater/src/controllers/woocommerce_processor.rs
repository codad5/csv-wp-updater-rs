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

#[derive(Debug, Serialize, Deserialize)]
struct WooCommerceProduct {
  // Core product details
  name: String,
  sku: String,
  regular_price: String,
  sale_price: Option<String>,
  description: String,
  short_description: String,

  // Categorization
  categories: Vec<Category>,
  tags: Vec<String>,

  // Images
  images: Vec<ProductImage>,

  // Variations support
  #[serde(skip_serializing_if = "Vec::is_empty")]
  variations: Vec<ProductVariation>,

  // Additional attributes
  attributes: Vec<ProductAttribute>,

  // Stock and shipping
  manage_stock: Option<bool>,
  stock_quantity: Option<i32>,
  shipping_class: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Category {
  id: Option<i32>,
  name: String,
  slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductImage {
  src: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductVariation {
  sku: String,
  regular_price: String,
  attributes: Vec<VariationAttribute>,
  #[serde(skip_serializing_if = "Option::is_none")]
  stock_quantity: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProductAttribute {
  name: String,
  position: Option<i32>,
  visible: Option<bool>,
  variation: Option<bool>,
  options: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VariationAttribute {
  name: String,
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
  woocommerce_client: Client,
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
      woocommerce_client,
      redis_client,
      progress: Arc::new(Mutex::new(ProcessingProgress::default())),
      base_url,
      consumer_key,
      consumer_secret,
    }
  }

  async fn process_csv(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;
    let mut redis_conn = self.redis_client.get_multiplexed_async_connection().await?;
    println!("Connected to Redis");
    println!("Processing CSV file: {}", file_path);
    // Reset progress
    let mut progress = self.progress.lock().await;
    *progress = ProcessingProgress::default();
    progress.total_rows = rdr.records().count();
    drop(progress);

    println!("progress tracking rows");

    // Reset reader
    let mut rdr = Reader::from_path(get_upload_path(file_path))?;

    // Group rows by parent SKU for handling variations
    let mut product_groups: HashMap<String, Vec<csv::StringRecord>> = HashMap::new();
    println!("Grouping products by SKU...");
    // println!("Total rows: {}", rdr.records().count());
    println!("Processing records...");
    // println!("All records: {:?}", rdr.records());
   let mut iter = rdr.records();
   while let Some(result) = iter.next() {
        println!("Processing record... counting");
        let record = result?;
        println!("Processing record: {:?}", record);
        let sku = record.get(0).unwrap_or("").to_string();
        let record_type = record.get(24).unwrap_or("").to_string();
        println!("Record type: {}", record_type);
        
        if record_type.contains("PRODUCT") {
            product_groups.entry(sku.clone()).or_default().push(record);
        } else if record_type.contains("MODEL") {
            let parent_sku = record.get(10).unwrap_or("").to_string();
            product_groups.entry(parent_sku).or_default().push(record);
        }
    }

    println!("Total product groups: {}", product_groups.len());
    println!(
      "Product groups: {:?}",
      product_groups.keys().collect::<Vec<_>>()
    );
    println!("Processing products...");

    // Process each product group
    for (parent_sku, records) in product_groups {
      // Separate main product and variations
      let main_product = records
        .iter()
        .find(|r| r.get(24).unwrap_or("").contains("PRODUCT"))
        .ok_or("No main product found")?;

      // Check Redis for existing SKU
      let exists: bool = redis_conn.hexists("products", &parent_sku).await?;

      if !exists {
        println!("Processing product: {}", parent_sku);
        // Prepare product data
        let product = self.build_product(main_product, &records)?;

        // Send to WooCommerce
        let response = self.upload_product(&product).await?;
        println!("Response: {:?}", response);

        if response.status().is_success() {
          // Store in Redis
          redis_conn
            .hset("products", &parent_sku, serde_json::to_string(&product)?)
            .await?;

          let mut progress = self.progress.lock().await;
          progress.successful_rows += 1;
          progress.new_entries += 1;
        } else {
          let mut progress = self.progress.lock().await;
          progress.failed_rows += 1;
        }
      }

      let mut progress = self.progress.lock().await;
      progress.processed_rows += 1;
    }

    Ok(())
  }

  fn build_product(
    &self,
    main_record: &csv::StringRecord,
    all_records: &[csv::StringRecord],
  ) -> Result<WooCommerceProduct, Box<dyn std::error::Error>> {
    // Extract main product details
    let sku = main_record.get(0).unwrap_or("").to_string();
    let title = main_record.get(7).unwrap_or("").to_string();
    let description = main_record.get(8).unwrap_or("").to_string();
    let short_description = main_record.get(9).unwrap_or("").to_string();
    let regular_price = main_record.get(14).unwrap_or("").to_string();
    let sale_price = main_record.get(16).map(|p| p.to_string());

    // Images
    let featured_image = main_record.get(1).unwrap_or("");
    let gallery_images = main_record
      .get(2)
      .unwrap_or("")
      .split('|')
      .collect::<Vec<_>>();
    let mut images = vec![];
    if !featured_image.is_empty() {
      images.push(ProductImage {
        src: featured_image.to_string(),
        name: Some(title.clone()),
        alt: Some(title.clone()),
      });
    }
    images.extend(
      gallery_images
        .iter()
        .filter(|img| !img.is_empty())
        .map(|img| ProductImage {
          src: img.to_string(),
          name: Some(title.clone()),
          alt: Some(title.clone()),
        }),
    );

    // Attributes
    let material = main_record.get(3).unwrap_or("");
    let brand = main_record.get(5).unwrap_or("");
    let mut attributes = vec![];
    if !material.is_empty() {
      attributes.push(ProductAttribute {
        name: "Material".to_string(),
        position: Some(1),
        visible: Some(true),
        variation: Some(false),
        options: material.split(',').map(|m| m.trim().to_string()).collect(),
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

    // Variations
    let variations = all_records
      .iter()
      .filter(|r| r.get(24).unwrap_or("").contains("MODEL"))
      .map(|record| ProductVariation {
        sku: record.get(0).unwrap_or("").to_string(),
        regular_price: record.get(14).unwrap_or("").to_string(),
        stock_quantity: record.get(15).and_then(|s| s.parse().ok()),
        attributes: vec![
          VariationAttribute {
            name: "Size".to_string(),
            option: record.get(18).unwrap_or("").to_string(),
          },
          VariationAttribute {
            name: "Color".to_string(),
            option: record.get(19).unwrap_or("").to_string(),
          },
        ],
      })
      .collect();

    // Categories
    let categories_str = main_record.get(12).unwrap_or("");
    let categories = categories_str
      .split('/')
      .filter(|c| !c.is_empty())
      .map(|cat| Category {
        id: None,
        name: cat.to_string(),
        slug: cat.to_lowercase().replace(' ', "-"),
      })
      .collect();

    Ok(WooCommerceProduct {
      name: title,
      sku,
      regular_price,
      sale_price,
      description,
      short_description,
      categories,
      tags: vec![], // Could extract from CSV if needed
      images,
      variations,
      attributes,
      manage_stock: Some(true),
      stock_quantity: main_record.get(15).and_then(|s| s.parse().ok()),
      shipping_class: main_record.get(20).map(|s| s.to_string()),
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
  file_path: String,
  base_url: String,
  consumer_key: String,
  consumer_secret: String,
) -> Result<(), String> {
  println!("Processing CSV: {}", file_path);
  println!("Base URL: {}", base_url);
  println!("Consumer Key: {}", consumer_key);
  let processor = WooCommerceProcessor::new(base_url, consumer_key, consumer_secret).await;
  println!("Processor created");
  println!("Processing CSV file...");
  match processor.process_csv(&file_path).await {
    Ok(_) => Ok(()),
    Err(e) => Err(format!("Error processing CSV: {}", e)),
  }
}