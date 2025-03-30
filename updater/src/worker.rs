use amiquip::Connection;
use crate::{controllers::queue_handler::RabbitMQFileProcessor, types::csv_field_woo_mapper::WordPressFieldMapping};
use crate::types::csv_field_woo_mapper::{default_priority, default_wordpress_field_mapping};
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NewFileProcessQueue {
    pub file: String,
    pub start_row: u32,
    pub row_count: u32,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_wordpress_field_mapping")]
    pub wordpress_field_mapping: WordPressFieldMapping,
}


pub async fn run_worker() -> Result<(), amiquip::Error> {
    let connection_url = std::env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");
    
//     // Connect to RabbitMQ server
    let connection: Connection = Connection::insecure_open(&connection_url)?;
    let mqservice = RabbitMQFileProcessor::new(connection);
    if let Err(e) = mqservice.listen_for_messages().await {
        println!("Error listening for messages: {}", e);
    }
    Ok(())
}