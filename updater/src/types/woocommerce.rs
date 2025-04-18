use serde::{Deserialize, Serialize};
use serde::{Serializer, Deserializer};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use super::csv_field_woo_mapper::AttributeMapping;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WooCommerceProduct {
    // Core product details
    #[serde(
        skip_serializing_if = "String::is_empty",
        serialize_with = "serialize_id_as_number",
        deserialize_with = "deserialize_id_as_string",
        default
    )]
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub sku: String,
    #[serde(rename = "type", skip_serializing_if = "String::is_empty", default)]
    pub type_: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub parent: String,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    featured: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<String>,
    
    // Categorization
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    categories: Vec<Category>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    tags: Vec<Category>,
    
    // Images
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    images: Vec<ProductImage>,
    
    // Variations support
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    variations: Vec<u64>,
    
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
    #[serde(skip_serializing_if = "Option::is_none")]
    stock_status: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none", 
        default,
        deserialize_with = "deserialize_optional_string"
    )]
    shipping_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    dimensions: Option<ProductDimension>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    meta_data:Vec<KeyValue>
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProductVariation {
    // Core product details
    #[serde(
        skip_serializing_if = "String::is_empty",
        serialize_with = "serialize_id_as_number",
        deserialize_with = "deserialize_id_as_string",
        default
    )]
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sku: String,
    #[serde(skip_serializing_if = "String::is_empty", default, skip_serializing)]
    pub parent: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    regular_price: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    sale_price: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attributes: Vec<VariationAttribute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stock_quantity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    image: Option<ProductImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stock_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<ProductDimension>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    meta_data:Vec<KeyValue>
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Category {
  id: Option<i32>,
  #[serde(skip_serializing_if = "String::is_empty")]
  name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  slug: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProductImage {
  #[serde(skip_serializing_if = "String::is_empty")]
  src: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  alt: Option<String>,
}



#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProductAttribute {
  #[serde(skip_serializing_if = "String::is_empty")]
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  visible: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  variation: Option<bool>,
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub options: Vec<String>,
}

impl ProductAttribute {
    pub fn new(name:&str, options: Vec<String>) -> Self {
        ProductAttribute {
            name: name.to_owned(),
            position: None,
            visible: Some(true),
            variation: Some(true),
            options
        }
    }

    
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct VariationAttribute {
  #[serde(skip_serializing_if = "String::is_empty")]
  pub name: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  pub option: String,
}


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProductDimension {
  #[serde(skip_serializing_if = "String::is_empty")]
  length: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  width: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  height: String,
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
        if !["simple", "grouped", "external", "variable",  "variation"].contains(&type_.as_str()) {
            type_  =  String::new(); // set to empty string if not valid
             // Return self if type is not valid
        } 
        // Validate stock status
        let all_stock_status_type = vec!["instock", "outofstock", "onbackorder"];
        let mut stock_status = merge_option(&self.stock_status, &other.stock_status);
        if !all_stock_status_type.contains(&self.stock_status.as_deref().unwrap_or("").to_lowercase().as_str()) {
            stock_status = Some(String::from("instock")); // Reset type if stock status is invalid
        }

        WooCommerceProduct {
            // Core product details
            name: merge_string(&self.name, &other.name),
            id: merge_string(&self.id, &other.id),
            sku: merge_string(&self.sku, &other.sku),
            type_,
            featured:false,
            regular_price: merge_string(&self.regular_price, &other.regular_price),
            sale_price: merge_option(&self.sale_price, &other.sale_price),
            description: merge_string(&self.description, &other.description),
            short_description: merge_string(&self.short_description, &other.short_description),
            weight: merge_option(&self.weight, &other.weight),
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
            stock_status,
            shipping_class: merge_option(&self.shipping_class, &other.shipping_class),
            dimensions: merge_option(&self.dimensions, &other.dimensions),
            meta_data: merge_vec(&self.meta_data, &other.meta_data),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() && self.is_main_product() {
            return Err("Product name is required".to_string());
        }
        if self.sku.is_empty() {
            return Err("Product SKU is required".to_string());
        }
        if self.regular_price.is_empty() && self.type_ == String::from("simple") {
            return Err("Product regular price is required".to_string());
        }
        // if self.description.is_empty() {
        //     return Err("Product description is required".to_string());
        // }
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

    pub fn get_attribute(&self) -> Vec<ProductAttribute> {
        self.attributes.clone()
    }

    pub fn get_attribute_mut(&mut self) -> &mut Vec<ProductAttribute> {
        &mut self.attributes
    }

    pub fn add_attribute(&mut self, attribute: ProductAttribute) {
        self.attributes.push(attribute);
    }


}


impl ProductVariation {
    /// Merges two ProductVariation instances, with values from `other` taking precedence
    /// over values from `self` when both exist and are not empty.
    /// 
    /// Returns a new ProductVariation instance with the merged values.
    pub fn merge(&self, other: &ProductVariation) -> ProductVariation {
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


        ProductVariation {
            // Core product details
            id: merge_string(&self.id, &other.id),
            sku: merge_string(&self.sku, &other.sku),
            regular_price: merge_string(&self.regular_price, &other.regular_price),
            sale_price: merge_string(&self.sale_price, &other.sale_price),
            description: merge_string(&self.description, &other.description),
            parent: merge_string(&self.parent, &other.parent),
            
            // Additional attributes
            attributes: merge_vec(&self.attributes, &other.attributes),
            stock_quantity: merge_option(&self.stock_quantity, &other.stock_quantity),
            shipping_class: merge_option(&self.shipping_class, &other.shipping_class),
            dimensions: merge_option(&self.dimensions, &other.dimensions),
            weight: merge_option(&self.weight, &other.weight),
            image: merge_option(&self.image, &other.image),
            stock_status: merge_option(&self.stock_status, &other.stock_status), 
            meta_data: merge_vec(&self.meta_data, &other.meta_data),
        }
    }

    pub fn set_parent(&mut self, parent: &String) {
        self.parent = parent.clone();
        println!("\x1b[38;5;220mUpdated child with ID {} and SKU {} parent ID to {}\x1b[0m", self.id, self.sku, parent);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.sku.is_empty() {
            return Err("Product SKU is required".to_string());
        }
        if self.regular_price.is_empty() {
            return Err("Product regular price is required".to_string());
        }
        // if self.description.is_empty() {
        //     return Err("Product description is required".to_string());
        // }
        Ok(())
    }

    // to check if the product has changed or not
    pub fn get_changes(&self, other: &ProductVariation) -> Vec<String> {
        let mut changes = Vec::new();
        

        if self.sku != other.sku {
            changes.push("sku".to_string());
        }
        
        if self.description != other.description {
            changes.push("description".to_string());
        }

        if self.attributes != other.attributes && !other.attributes.is_empty() {
            changes.push("attributes".to_string());
        }
        
        if self.stock_quantity != other.stock_quantity {
            changes.push("stock_quantity".to_string());
        }
        
        if self.shipping_class != other.shipping_class {
            changes.push("shipping_class".to_string());
        }
        
        println!("Product with SKU {} has {} changes: {:?}", 
                self.sku, changes.len(), changes);
        
        changes
    }

    pub fn has_changed(&self, other: &ProductVariation) -> bool {
        !self.get_changes(other).is_empty()
    }

    // a method to check if main product or a variation 
    pub fn is_main_product(&self) -> bool {
        false
    }
    
    pub fn get_attribute(&self) -> Vec<VariationAttribute> {
        self.attributes.clone()
    }
}



pub enum WooProduct {
    Product(WooCommerceProduct),
    Variation(ProductVariation),
}

pub fn woo_build_product(product: &HashMap<String, String>, attribute: &HashMap<String, AttributeMapping>) -> Option<WooProduct> {
    let binding = "".to_string();
    let parent = product.get("parent_id").unwrap_or(&binding).trim();
    let sku = product.get("sku").unwrap_or(&binding).trim();
    println!("\x1b[38;5;208mProduct Debug Info: {{ SKU: {}, Parent SKU: {}, Is Child: {} }}\x1b[0m",
        sku,
        parent,
        !parent.is_empty() && parent != sku
    );
    if parent.is_empty() || parent  == sku {
        // Build a main product
        match woo_product_builder(product, attribute) {
            Ok(product) => Some(WooProduct::Product(product)),
            Err(_) => None,
        }
    } else {
        // Build a variation
        match woo_product_variation_builder(product, attribute) {
            Ok(variation) => Some(WooProduct::Variation(variation)),
            Err(_) => None,
        }
    }
}




pub fn  woo_product_builder(
    product: &HashMap<String, String>,
    attribute_map: &HashMap<String, AttributeMapping>
) -> Result<WooCommerceProduct, Box<dyn std::error::Error + Send + Sync>> {
    
    
    let get_value = |key: &str| -> String {
        product.get(key).unwrap_or(&"".to_string()).trim().to_string()
    };

    let id = get_value("id");
    let sku = get_value("sku");
    let mut type_ = get_value("type");
    let name = get_value("name");
    let description = get_value("description");
    let short_description = get_value("short_description");
    let regular_price = get_value("regular_price");
    let sale_price = get_value("sale_price");
    let mut stock_status = get_value("stock_status");
    let parent = String::new();
    let feature_words = ["yes", "true", "1"];
    let featured = feature_words.contains(&get_value("featured").to_lowercase().as_str());
    let dimensions =  build_product_dimensions(product);

    // if parent is not empty then type is variation
    if !parent.is_empty() {
        type_ = "variation".to_string();
    } else {
        // check if type is in array of simple, grouped, external and variable
        if !["simple", "grouped", "external", "variable", "variation"].contains(&type_.to_lowercase().as_str()) {
            type_  =  String::new(); // set to empty string if not valid
        } 
    }

    let all_stock_status_type = vec!["instock", "outofstock", "onbackorder"];
    if !all_stock_status_type.contains(&stock_status.as_str()) {
        stock_status = String::from("instock"); // Reset type if stock status is invalid
    }
      
      // Handle categories
      println!("categories {:?}",  get_value("category_ids"));
      let categories: Vec<Category> = get_value("category_ids")
          .split('|')
          .filter(|c| !c.trim().is_empty())
          .map(|cat| Category {
              id: cat.trim().parse().ok(),
              name: cat.trim().to_string(),
              slug: cat.trim().to_lowercase().replace(' ', "-"),
          })
          .collect();

        let tags: Vec<Category> = get_value("tag_ids")
          .split('|')
          .filter(|c| !c.trim().is_empty())
          .map(|cat| Category {
              id: cat.trim().parse().ok(),
              name: cat.trim().to_string(),
              slug: cat.trim().to_lowercase().replace(' ', "-"),
          })
          .collect();

      // Handle images
      let main_image = get_value("images");
      let featured_image = get_value("featured_image");
      let gallery_images: Vec<_> = if !main_image.is_empty() {
          main_image.split('|')
              .filter(|img| !img.trim().is_empty())
              .collect()
      } else {
          vec![]
      };

    println!("\x1b[38;5;45mImages for Product ID: {}, SKU: {}: {:?}\x1b[0m", id, sku, gallery_images);
      
      let mut images = vec![];
      let mut meta_data = vec![];
      
      if !featured_image.is_empty() {
        meta_data.push(KeyValue{
            key:"fifu_list_url".to_owned(), 
            value: serde_json::Value::String(featured_image.clone())
        });
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
      for (key, value) in attribute_map.iter() {
        if !value.column.is_empty() {
            attributes.push(ProductAttribute {
                name: key.clone(),
                position: None,
                visible: Some(value.variable),
                variation: Some(type_ != "simple"),
                options: value.column.split(',')
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect(),
            });
        }
    }


    println!("variation attribute : {:?}", attributes);

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

    println!("\x1b[38;5;43mShipping Class for Product ID: {}, SKU: {}: {}\x1b[0m", id, sku, shipping_class_option.clone().unwrap_or_default());
      let weight = get_value("weight");
      let weight_option = if weight.is_empty() {
            None
        } else {
            Some(weight)
        };
        
        
        println!("\x1b[38;5;231mImages for Product ID: {}, SKU: {}: {:?}\x1b[0m", id, sku, images);
      Ok(WooCommerceProduct {
          id,
          name,
          sku,
          type_,
          featured,
          parent,
          regular_price,
          sale_price: sale_price_option,
          description,
          short_description,
          weight:weight_option,
          categories,
          tags,
          images,
          variations: vec![], // You can implement variations if needed
          attributes,
          manage_stock: if stock_quantity.is_some() { Some(true) } else { None },
          stock_quantity,
          shipping_class: shipping_class_option,
          dimensions,
          meta_data, 
          stock_status:Some(stock_status)
      })
  }


pub fn woo_product_variation_builder(
    product: &HashMap<String, String>,
    attribute_map: &HashMap<String, AttributeMapping>,
) -> Result<ProductVariation, Box<dyn std::error::Error + Send + Sync>> {
    let get_value = |key: &str| -> String {
        product.get(key).unwrap_or(&"".to_string()).trim().to_string()
    };

    let id = get_value("id");
    let sku = get_value("sku");
    let description = get_value("description");
    let regular_price = get_value("regular_price");
    let sale_price = get_value("sale_price");
    let parent = get_value("parent_id");
    
    // Handle attributes for variations
    let mut attributes = Vec::new();
    
    // Look for variation attributes in the product hashmap
    // This assumes that variation attributes are stored with keys like "attribute_color", "attribute_size", etc.
    // for (key, value) in product.iter() {
    //     if key.starts_with("attribute_") && !value.is_empty() {
    //         let attr_name = key.trim_start_matches("attribute_").to_string();
    //         attributes.push(VariationAttribute {
    //             name: attr_name,
    //             option: value.clone(),
    //         });
    //     }
    // }
    
    // Handle specific variation attributes if they don't follow the standard pattern
    let color = get_value("color");
    let size = get_value("size");
    
    // if !color.is_empty() && !attributes.iter().any(|a| a.name == "Color") {
    //     attributes.push(VariationAttribute {
    //         name: "Color".to_string(),
    //         option: color,
    //     });
    // }
    
    // if !size.is_empty() && !attributes.iter().any(|a| a.name == "Size") {
    //     attributes.push(VariationAttribute {
    //         name: "Size".to_string(),
    //         option: size,
    //     });
    // }

    let main_image = get_value("images");
    let featured_image = get_value("featured_image");

    let gallery_images: Vec<_> = if !main_image.is_empty() {
          main_image.split('|')
              .filter(|img| !img.trim().is_empty())
              .collect()
      } else {
          vec![]
      };
    
    println!("\x1b[38;5;45mImages for ProductVariation ID: {}, SKU: {}: {:?}\x1b[0m", id, sku, gallery_images);
    let mut images = vec![];
    let mut meta_data = vec![];
    
    if !featured_image.is_empty() {
    meta_data.push(KeyValue{
        key:"fifu_list_url".to_owned(), 
        value: serde_json::Value::String(featured_image.clone())

    });
    images.push(ProductImage {
        src: featured_image.clone(),
        name: None,
        alt: None
    });
    }
    
    images.extend(gallery_images.iter().map(|img| ProductImage {
        src: img.trim().to_string(),
        name: None,
        alt: None
    }));

    for (key, value) in attribute_map.iter() {
            if !value.column.is_empty() {
                attributes.push(VariationAttribute {
                    name: key.clone(),
                    option: value.column.to_owned()
                });
            }
        }

    println!("variation attribute : {:?}", attributes);
    
    // Handle stock quantity and status
    let stock_quantity = get_value("stock_quantity").parse().ok();
    let mut stock_status = get_value("stock_status");
    let all_stock_status_type = vec!["instock", "outofstock", "onbackorder"];
    if !all_stock_status_type.contains(&stock_status.as_str()) {
        stock_status = "instock".to_string(); // Default to "instock" if not valid
    }

    
    // Handle weight
    let weight = get_value("weight");
    let weight_option = if weight.is_empty() {
        None
    } else {
        Some(weight)
    };
    
    // Handle shipping class
    let shipping_class = get_value("shipping_class_id");
    let shipping_class_option = if shipping_class.is_empty() {
        None
    } else {
        Some(shipping_class)
    };
    
    // Handle dimensions using the build_product_dimensions function
    let dimensions =  build_product_dimensions(product);
     println!("\x1b[38;5;231mImages for Product ID: {}, SKU: {}: {:?}\x1b[0m", id, sku, images);
    Ok(ProductVariation {
        id,
        sku,
        parent,
        description,
        regular_price,
        sale_price,
        attributes,
        stock_quantity,
        stock_status:Some(stock_status),
        weight: weight_option,
        shipping_class: shipping_class_option,
        dimensions,
        image: images.first().cloned(), 
        meta_data
    })
}




/// Builds a ProductDimension from a HashMap of product data
pub fn build_product_dimensions(
    product: &HashMap<String, String>,
) -> Option<ProductDimension> {
    let get_value = |key: &str| -> String {
        product.get(key).unwrap_or(&"".to_string()).trim().to_string()
    };
    
    let length = get_value("length");
    let width = get_value("width");
    let height = get_value("height");
    
    // Only create a dimensions object if at least one dimension is provided
    if length.is_empty() && width.is_empty() && height.is_empty() {
        return None;
    }
    
    Some(ProductDimension {
        length,
        width,
        height,
    })
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


