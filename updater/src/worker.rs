use crate::types::csv_field_woo_mapper::{default_priority, default_wordpress_field_mapping};
use crate::{
    controllers::queue_handler::RabbitMQFileProcessor,
    types::csv_field_woo_mapper::WordPressFieldMapping,
};
use amiquip::Connection;
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NewFileProcessQueue {
    pub file: String,
    pub start_row: u32,
    pub row_count: u32,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_wordpress_field_mapping")]
    pub wordpress_field_mapping: WordPressFieldMapping,
    #[serde(default)]
    pub is_new_upload: bool,
    #[serde(default)]
    pub site_details: SiteDetails,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32, // products per batch before delay
    #[serde(default = "default_batch_delay_minutes")]
    pub batch_delay_minutes: u32, // delay in minutes between batches
    #[serde(default = "default_dry_run")]
    pub dry_run: bool, // new field for dry run mode
}

pub fn default_batch_delay_minutes() -> u32 {
    5 // default to 5 minutes
}

pub fn default_batch_size() -> u32 {
    50 // default to 50 products per batch
}

pub fn default_dry_run() -> bool {
    false // default to false (normal mode)
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct SiteDetails {
    pub key: String,
    pub secret: String,
    pub url: String,
    pub name: String,
}

pub async fn run_worker() -> Result<(), amiquip::Error> {
    let connection_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");

    //     // Connect to RabbitMQ server
    let connection: Connection = Connection::insecure_open(&connection_url)?;
    let mqservice = RabbitMQFileProcessor::new(connection);
    if let Err(e) = mqservice.listen_for_messages().await {
        println!("Error listening for messages: {}", e);
    };
    println!("Worker finished processing messages.");
    Ok(())
}
