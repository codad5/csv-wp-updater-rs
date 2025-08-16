use crate::types::woocommerce::{ProductVariation, WooCommerceProduct};

#[derive(Debug, Clone)]
pub struct ProductBatch {
    products: Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
    total_product_count: usize,
}

impl ProductBatch {
    pub fn new() -> Self {
        Self {
            products: Vec::new(),
            total_product_count: 0,
        }
    }
    
    pub fn add_product_family(&mut self, parent: WooCommerceProduct, children: Vec<ProductVariation>) {
        let family_size = 1 + children.len(); // parent + children
        self.total_product_count += family_size;
        self.products.push((parent, children));
    }
    
    pub fn would_exceed_limit(&self, family_size: usize, batch_size: usize) -> bool {
        // If this batch is empty, always allow (even if family exceeds batch_size)
        if self.products.is_empty() {
            return false;
        }
        // If adding this family would exceed the batch size
        self.total_product_count + family_size > batch_size
    }
    
    pub fn get_products(&self) -> &Vec<(WooCommerceProduct, Vec<ProductVariation>)> {
        &self.products
    }
    
    pub fn get_total_count(&self) -> usize {
        self.total_product_count
    }
}

pub fn create_product_batches(
    products_with_children: Vec<(WooCommerceProduct, Vec<ProductVariation>)>,
    batch_size: usize,
) -> Vec<ProductBatch> {
    let mut batches: Vec<ProductBatch> = Vec::new();
    let mut current_batch = ProductBatch::new();
    
    for (parent, children) in products_with_children {
        let family_size = 1 + children.len(); // parent + children count
        
        // If adding this family would exceed batch size and current batch is not empty
        if current_batch.would_exceed_limit(family_size, batch_size) {
            // Start a new batch
            batches.push(current_batch);
            current_batch = ProductBatch::new();
        }
        
        // Add the family to current batch
        current_batch.add_product_family(parent, children);
    }
    
    // Don't forget the last batch if it has products
    if !current_batch.products.is_empty() {
        batches.push(current_batch);
    }
    
    batches
}