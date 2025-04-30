export interface ProcessOptions {
    rowCount?: number;
    startRow?: number;
    priority?: 0 | 1 | 2; // 0 = low, 1 = normal, 2 = high
    is_new_upload?: boolean;
    wordpress_field_mapping?: WordPressFieldMapping;
    siteDetails: {
        key: string;
        secret: string;
        url: string;
        name: string;
    }
}

export interface WordPressFieldMapping {
    id?: string;
    type?: string;
    sku?: string;
    global_unique_id?: string;
    name?: string;
    published?: string;
    featured?: string;
    catalog_visibility?: string;
    short_description?: string;
    description?: string;
    regular_price?: string;
    sale_price?: string;
    date_on_sale_from?: string;
    date_on_sale_to?: string;
    tax_status?: string;
    tax_class?: string;
    stock_status?: string;
    manage_stock?: string;
    stock_quantity?: string;
    backorders?: string;
    low_stock_amount?: string;
    sold_individually?: string;
    weight?: string;
    length?: string;
    width?: string;
    height?: string;
    category_ids?: string;
    tag_ids?: string;
    tag_ids_spaces?: string;
    shipping_class_id?: string;
    images?: string;
    featured_image?: string;
    parent_id?: string;
    upsell_ids?: string;
    cross_sell_ids?: string;
    grouped_products?: string;
    product_url?: string;
    button_text?: string;
    download_limit?: string;
    download_expiry?: string;
    reviews_allowed?: string;
    purchase_note?: string;
    menu_order?: string;
    brand_ids?: string;
    attributes?: {
        [key: string]: {
            column: string, 
            variable:boolean
        };
    }

}
