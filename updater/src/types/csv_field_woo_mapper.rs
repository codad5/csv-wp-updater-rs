use std::collections::HashMap;


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WordPressFieldMapping {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub sku: Option<String>,
    pub global_unique_id: Option<String>,
    pub name: Option<String>,
    pub published: Option<String>,
    pub featured: Option<String>,
    pub catalog_visibility: Option<String>,
    pub short_description: Option<String>,
    pub description: Option<String>,
    pub regular_price: Option<String>,
    pub sale_price: Option<String>,
    pub date_on_sale_from: Option<String>,
    pub date_on_sale_to: Option<String>,
    pub tax_status: Option<String>,
    pub tax_class: Option<String>,
    pub stock_status: Option<String>,
    pub stock_quantity: Option<String>,
    pub backorders: Option<String>,
    pub low_stock_amount: Option<String>,
    pub sold_individually: Option<String>,
    pub weight: Option<String>,
    pub length: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub category_ids: Option<String>,
    pub tag_ids: Option<String>,
    pub tag_ids_spaces: Option<String>,
    pub shipping_class_id: Option<String>,
    pub images: Option<String>,
    pub parent_id: Option<String>,
    pub upsell_ids: Option<String>,
    pub cross_sell_ids: Option<String>,
    pub grouped_products: Option<String>,
    pub product_url: Option<String>,
    pub button_text: Option<String>,
    pub download_limit: Option<String>,
    pub download_expiry: Option<String>,
    pub reviews_allowed: Option<String>,
    pub purchase_note: Option<String>,
    pub menu_order: Option<String>,
    pub brand_ids: Option<String>,
    // Updated attribute field with the new structure
    #[serde(default)]
    pub attributes: HashMap<String, AttributeMapping>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AttributeMapping {
    pub column: String,
    pub variable: bool,
}

pub fn default_priority() -> u8 {
    1
}

pub fn default_wordpress_field_mapping() -> WordPressFieldMapping {
    WordPressFieldMapping {
        id: Some("ID FROM wp".to_string()),
        type_: Some("Product Type".to_string()),
        sku: Some("SKU".to_string()),
        global_unique_id: Some("Product Attribute: EAN".to_string()),
        name: Some("Title".to_string()),
        published: None,
        featured: Some("Featured Image".to_string()),
        catalog_visibility: Some("Catalog Visibility".to_string()),
        short_description: Some("Short Description".to_string()),
        description: Some("Description".to_string()),
        regular_price: Some("Regular Price".to_string()),
        sale_price: Some("Sale Price".to_string()),
        date_on_sale_from: None,
        date_on_sale_to: None,
        tax_status: None,
        tax_class: None,
        stock_status: None,
        stock_quantity: Some("Stock".to_string()),
        backorders: None,
        low_stock_amount: None,
        sold_individually: None,
        weight: Some("Weight".to_string()),
        length: None,
        width: None,
        height: None,
        category_ids: Some("Categories".to_string()),
        tag_ids: Some("Tags".to_string()),
        tag_ids_spaces: None,
        shipping_class_id: Some("Shipping Class".to_string()),
        images: Some("Product Image Gallery".to_string()),
        parent_id: Some("Parent".to_string()),
        upsell_ids: None,
        cross_sell_ids: None,
        grouped_products: None,
        product_url: Some("Slug".to_string()),
        button_text: None,
        download_limit: None,
        download_expiry: None,
        reviews_allowed: None,
        purchase_note: None,
        menu_order: None,
        brand_ids: Some("Product Attribute: Brand".to_string()),
        // Initialize the new attributes HashMap as empty
        attributes: HashMap::new(),
    }
}


impl WordPressFieldMapping {
    /// Converts the WordPressFieldMapping struct to a HashMap for easier field access
    pub fn to_hashmap(&self) -> HashMap<String, Option<String>> {
        let mut map = HashMap::new();
        
        map.insert("id".to_string(), self.id.clone());
        map.insert("type".to_string(), self.type_.clone());
        map.insert("sku".to_string(), self.sku.clone());
        map.insert("global_unique_id".to_string(), self.global_unique_id.clone());
        map.insert("name".to_string(), self.name.clone());
        map.insert("published".to_string(), self.published.clone());
        map.insert("featured".to_string(), self.featured.clone());
        map.insert("catalog_visibility".to_string(), self.catalog_visibility.clone());
        map.insert("short_description".to_string(), self.short_description.clone());
        map.insert("description".to_string(), self.description.clone());
        map.insert("regular_price".to_string(), self.regular_price.clone());
        map.insert("sale_price".to_string(), self.sale_price.clone());
        map.insert("date_on_sale_from".to_string(), self.date_on_sale_from.clone());
        map.insert("date_on_sale_to".to_string(), self.date_on_sale_to.clone());
        map.insert("tax_status".to_string(), self.tax_status.clone());
        map.insert("tax_class".to_string(), self.tax_class.clone());
        map.insert("stock_status".to_string(), self.stock_status.clone());
        map.insert("stock_quantity".to_string(), self.stock_quantity.clone());
        map.insert("backorders".to_string(), self.backorders.clone());
        map.insert("low_stock_amount".to_string(), self.low_stock_amount.clone());
        map.insert("sold_individually".to_string(), self.sold_individually.clone());
        map.insert("weight".to_string(), self.weight.clone());
        map.insert("length".to_string(), self.length.clone());
        map.insert("width".to_string(), self.width.clone());
        map.insert("height".to_string(), self.height.clone());
        map.insert("category_ids".to_string(), self.category_ids.clone());
        map.insert("tag_ids".to_string(), self.tag_ids.clone());
        map.insert("tag_ids_spaces".to_string(), self.tag_ids_spaces.clone());
        map.insert("shipping_class_id".to_string(), self.shipping_class_id.clone());
        map.insert("images".to_string(), self.images.clone());
        map.insert("parent_id".to_string(), self.parent_id.clone());
        map.insert("upsell_ids".to_string(), self.upsell_ids.clone());
        map.insert("cross_sell_ids".to_string(), self.cross_sell_ids.clone());
        map.insert("grouped_products".to_string(), self.grouped_products.clone());
        map.insert("product_url".to_string(), self.product_url.clone());
        map.insert("button_text".to_string(), self.button_text.clone());
        map.insert("download_limit".to_string(), self.download_limit.clone());
        map.insert("download_expiry".to_string(), self.download_expiry.clone());
        map.insert("reviews_allowed".to_string(), self.reviews_allowed.clone());
        map.insert("purchase_note".to_string(), self.purchase_note.clone());
        map.insert("menu_order".to_string(), self.menu_order.clone());
        map.insert("brand_ids".to_string(), self.brand_ids.clone());
        
        // For attributes, convert each key/value pair to "attribute_{key}" for consistent access
        for (key, value) in &self.attributes {
            map.insert(format!("attribute_{}", key), Some(value.clone().column));
        }
        
        map
    }
    
    /// Get a field mapping value by property name
    pub fn get_field(&self, property_name: &str) -> Option<&String> {
        // Check if it's an attribute request
        if property_name.starts_with("attribute_") {
            // Extract the attribute name from the property_name
            let attr_name = property_name.strip_prefix("attribute_").unwrap();
            return self.attributes.get(attr_name).map(|attr| &attr.column)
        }
        
        // Regular fields
        match property_name {
            "id" => self.id.as_ref(),
            "type" => self.type_.as_ref(),
            "sku" => self.sku.as_ref(),
            "global_unique_id" => self.global_unique_id.as_ref(),
            "name" => self.name.as_ref(),
            "published" => self.published.as_ref(),
            "featured" => self.featured.as_ref(),
            "catalog_visibility" => self.catalog_visibility.as_ref(),
            "short_description" => self.short_description.as_ref(),
            "description" => self.description.as_ref(),
            "regular_price" => self.regular_price.as_ref(),
            "sale_price" => self.sale_price.as_ref(),
            "date_on_sale_from" => self.date_on_sale_from.as_ref(),
            "date_on_sale_to" => self.date_on_sale_to.as_ref(),
            "tax_status" => self.tax_status.as_ref(),
            "tax_class" => self.tax_class.as_ref(),
            "stock_status" => self.stock_status.as_ref(),
            "stock_quantity" => self.stock_quantity.as_ref(),
            "backorders" => self.backorders.as_ref(),
            "low_stock_amount" => self.low_stock_amount.as_ref(),
            "sold_individually" => self.sold_individually.as_ref(),
            "weight" => self.weight.as_ref(),
            "length" => self.length.as_ref(),
            "width" => self.width.as_ref(),
            "height" => self.height.as_ref(),
            "category_ids" => self.category_ids.as_ref(),
            "tag_ids" => self.tag_ids.as_ref(),
            "tag_ids_spaces" => self.tag_ids_spaces.as_ref(),
            "shipping_class_id" => self.shipping_class_id.as_ref(),
            "images" => self.images.as_ref(),
            "parent_id" => self.parent_id.as_ref(),
            "upsell_ids" => self.upsell_ids.as_ref(),
            "cross_sell_ids" => self.cross_sell_ids.as_ref(),
            "grouped_products" => self.grouped_products.as_ref(),
            "product_url" => self.product_url.as_ref(),
            "button_text" => self.button_text.as_ref(),
            "download_limit" => self.download_limit.as_ref(),
            "download_expiry" => self.download_expiry.as_ref(),
            "reviews_allowed" => self.reviews_allowed.as_ref(),
            "purchase_note" => self.purchase_note.as_ref(),
            "menu_order" => self.menu_order.as_ref(),
            "brand_ids" => self.brand_ids.as_ref(),
            _ => None,
        }
    }
    
    /// Get all non-None field mappings as a HashMap of property name to field value
    pub fn get_active_mappings(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        if let Some(val) = &self.id { map.insert("id".to_string(), val.clone()); }
        if let Some(val) = &self.type_ { map.insert("type".to_string(), val.clone()); }
        if let Some(val) = &self.sku { map.insert("sku".to_string(), val.clone()); }
        if let Some(val) = &self.global_unique_id { map.insert("global_unique_id".to_string(), val.clone()); }
        if let Some(val) = &self.name { map.insert("name".to_string(), val.clone()); }
        if let Some(val) = &self.published { map.insert("published".to_string(), val.clone()); }
        if let Some(val) = &self.featured { map.insert("featured".to_string(), val.clone()); }
        if let Some(val) = &self.catalog_visibility { map.insert("catalog_visibility".to_string(), val.clone()); }
        if let Some(val) = &self.short_description { map.insert("short_description".to_string(), val.clone()); }
        if let Some(val) = &self.description { map.insert("description".to_string(), val.clone()); }
        if let Some(val) = &self.regular_price { map.insert("regular_price".to_string(), val.clone()); }
        if let Some(val) = &self.sale_price { map.insert("sale_price".to_string(), val.clone()); }
        if let Some(val) = &self.date_on_sale_from { map.insert("date_on_sale_from".to_string(), val.clone()); }
        if let Some(val) = &self.date_on_sale_to { map.insert("date_on_sale_to".to_string(), val.clone()); }
        if let Some(val) = &self.tax_status { map.insert("tax_status".to_string(), val.clone()); }
        if let Some(val) = &self.tax_class { map.insert("tax_class".to_string(), val.clone()); }
        if let Some(val) = &self.stock_status { map.insert("stock_status".to_string(), val.clone()); }
        if let Some(val) = &self.stock_quantity { map.insert("stock_quantity".to_string(), val.clone()); }
        if let Some(val) = &self.backorders { map.insert("backorders".to_string(), val.clone()); }
        if let Some(val) = &self.low_stock_amount { map.insert("low_stock_amount".to_string(), val.clone()); }
        if let Some(val) = &self.sold_individually { map.insert("sold_individually".to_string(), val.clone()); }
        if let Some(val) = &self.weight { map.insert("weight".to_string(), val.clone()); }
        if let Some(val) = &self.length { map.insert("length".to_string(), val.clone()); }
        if let Some(val) = &self.width { map.insert("width".to_string(), val.clone()); }
        if let Some(val) = &self.height { map.insert("height".to_string(), val.clone()); }
        if let Some(val) = &self.category_ids { map.insert("category_ids".to_string(), val.clone()); }
        if let Some(val) = &self.tag_ids { map.insert("tag_ids".to_string(), val.clone()); }
        if let Some(val) = &self.tag_ids_spaces { map.insert("tag_ids_spaces".to_string(), val.clone()); }
        if let Some(val) = &self.shipping_class_id { map.insert("shipping_class_id".to_string(), val.clone()); }
        if let Some(val) = &self.images { map.insert("images".to_string(), val.clone()); }
        if let Some(val) = &self.parent_id { map.insert("parent_id".to_string(), val.clone()); }
        if let Some(val) = &self.upsell_ids { map.insert("upsell_ids".to_string(), val.clone()); }
        if let Some(val) = &self.cross_sell_ids { map.insert("cross_sell_ids".to_string(), val.clone()); }
        if let Some(val) = &self.grouped_products { map.insert("grouped_products".to_string(), val.clone()); }
        if let Some(val) = &self.product_url { map.insert("product_url".to_string(), val.clone()); }
        if let Some(val) = &self.button_text { map.insert("button_text".to_string(), val.clone()); }
        if let Some(val) = &self.download_limit { map.insert("download_limit".to_string(), val.clone()); }
        if let Some(val) = &self.download_expiry { map.insert("download_expiry".to_string(), val.clone()); }
        if let Some(val) = &self.reviews_allowed { map.insert("reviews_allowed".to_string(), val.clone()); }
        if let Some(val) = &self.purchase_note { map.insert("purchase_note".to_string(), val.clone()); }
        if let Some(val) = &self.menu_order { map.insert("menu_order".to_string(), val.clone()); }
        if let Some(val) = &self.brand_ids { map.insert("brand_ids".to_string(), val.clone()); }
        
        // Add all attributes with attribute_ prefix
        for (key, val) in &self.attributes {
            map.insert(format!("attribute_{}", key), val.clone().column);
        }
        
        map
    }
    
    /// Create a reverse mapping: CSV field name to property name
    pub fn get_reverse_mapping(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        if let Some(val) = &self.id { map.insert(val.clone(), "id".to_string()); }
        if let Some(val) = &self.type_ { map.insert(val.clone(), "type".to_string()); }
        if let Some(val) = &self.sku { map.insert(val.clone(), "sku".to_string()); }
        if let Some(val) = &self.global_unique_id { map.insert(val.clone(), "global_unique_id".to_string()); }
        if let Some(val) = &self.name { map.insert(val.clone(), "name".to_string()); }
        if let Some(val) = &self.published { map.insert(val.clone(), "published".to_string()); }
        if let Some(val) = &self.featured { map.insert(val.clone(), "featured".to_string()); }
        if let Some(val) = &self.catalog_visibility { map.insert(val.clone(), "catalog_visibility".to_string()); }
        if let Some(val) = &self.short_description { map.insert(val.clone(), "short_description".to_string()); }
        if let Some(val) = &self.description { map.insert(val.clone(), "description".to_string()); }
        if let Some(val) = &self.regular_price { map.insert(val.clone(), "regular_price".to_string()); }
        if let Some(val) = &self.sale_price { map.insert(val.clone(), "sale_price".to_string()); }
        if let Some(val) = &self.date_on_sale_from { map.insert(val.clone(), "date_on_sale_from".to_string()); }
        if let Some(val) = &self.date_on_sale_to { map.insert(val.clone(), "date_on_sale_to".to_string()); }
        if let Some(val) = &self.tax_status { map.insert(val.clone(), "tax_status".to_string()); }
        if let Some(val) = &self.tax_class { map.insert(val.clone(), "tax_class".to_string()); }
        if let Some(val) = &self.stock_status { map.insert(val.clone(), "stock_status".to_string()); }
        if let Some(val) = &self.stock_quantity { map.insert(val.clone(), "stock_quantity".to_string()); }
        if let Some(val) = &self.backorders { map.insert(val.clone(), "backorders".to_string()); }
        if let Some(val) = &self.low_stock_amount { map.insert(val.clone(), "low_stock_amount".to_string()); }
        if let Some(val) = &self.sold_individually { map.insert(val.clone(), "sold_individually".to_string()); }
        if let Some(val) = &self.weight { map.insert(val.clone(), "weight".to_string()); }
        if let Some(val) = &self.length { map.insert(val.clone(), "length".to_string()); }
        if let Some(val) = &self.width { map.insert(val.clone(), "width".to_string()); }
        if let Some(val) = &self.height { map.insert(val.clone(), "height".to_string()); }
        if let Some(val) = &self.category_ids { map.insert(val.clone(), "category_ids".to_string()); }
        if let Some(val) = &self.tag_ids { map.insert(val.clone(), "tag_ids".to_string()); }
        if let Some(val) = &self.tag_ids_spaces { map.insert(val.clone(), "tag_ids_spaces".to_string()); }
        if let Some(val) = &self.shipping_class_id { map.insert(val.clone(), "shipping_class_id".to_string()); }
        if let Some(val) = &self.images { map.insert(val.clone(), "images".to_string()); }
        if let Some(val) = &self.parent_id { map.insert(val.clone(), "parent_id".to_string()); }
        if let Some(val) = &self.upsell_ids { map.insert(val.clone(), "upsell_ids".to_string()); }
        if let Some(val) = &self.cross_sell_ids { map.insert(val.clone(), "cross_sell_ids".to_string()); }
        if let Some(val) = &self.grouped_products { map.insert(val.clone(), "grouped_products".to_string()); }
        if let Some(val) = &self.product_url { map.insert(val.clone(), "product_url".to_string()); }
        if let Some(val) = &self.button_text { map.insert(val.clone(), "button_text".to_string()); }
        if let Some(val) = &self.download_limit { map.insert(val.clone(), "download_limit".to_string()); }
        if let Some(val) = &self.download_expiry { map.insert(val.clone(), "download_expiry".to_string()); }
        if let Some(val) = &self.reviews_allowed { map.insert(val.clone(), "reviews_allowed".to_string()); }
        if let Some(val) = &self.purchase_note { map.insert(val.clone(), "purchase_note".to_string()); }
        if let Some(val) = &self.menu_order { map.insert(val.clone(), "menu_order".to_string()); }
        if let Some(val) = &self.brand_ids { map.insert(val.clone(), "brand_ids".to_string()); }
        
        // Note: We don't include attributes in the reverse mapping as they're dynamic
        // and not part of the fixed struct fields
        
        map
    }
    
    /// Helper method to add a new attribute
    pub fn add_attribute(&mut self, key: String, value: AttributeMapping) {
        self.attributes.insert(key, value);
    }
    
    /// Helper method to get an attribute by name
    pub fn get_attribute(&self, key: &str) -> Option<&AttributeMapping> {
        self.attributes.get(key)
    }
    
    /// Helper method to remove an attribute
    pub fn remove_attribute(&mut self, key: &str) -> Option<AttributeMapping> {
        self.attributes.remove(key)
    }
    
    /// Helper method to get all attributes
    pub fn get_all_attributes(&self) -> &HashMap<String, AttributeMapping> {
        &self.attributes
    }

    pub fn get_inverted_attribute(&self) -> HashMap<String, AttributeMapping> {
        let inverted: HashMap<String, AttributeMapping> = self.attributes
        .iter()
        .map(|(k, v)| (v.clone(), k.clone()))
        .collect();

        return inverted;
    }


}