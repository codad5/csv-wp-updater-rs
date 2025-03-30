use amiquip::Connection;
use crate::controllers::queue_handler::RabbitMQFileProcessor;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct NewFileProcessQueue {
    pub file: String,
    pub start_row: u32,
    pub row_count: u32,
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