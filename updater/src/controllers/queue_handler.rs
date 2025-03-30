use std::{sync::Arc};
use amiquip::{Connection, ConsumerMessage, ConsumerOptions, Delivery, QueueDeclareOptions, Result as AmiqpResult};
use futures_lite::StreamExt;
use tokio::{sync::Semaphore, task};


use crate::{controllers::woocommerce_processor::process_woocommerce_csv, libs::redis::{get_redis_client, mark_model_as_completed, mark_model_as_failed, update_model_progress}, worker::NewFileProcessQueue};

pub struct RabbitMQFileProcessor {
    rabbit_mq_conn: Connection,
}

impl RabbitMQFileProcessor {
    pub fn new(rabbit_mq_conn: Connection) -> Self {
        Self {
            rabbit_mq_conn,
        }
    }

    pub async fn listen_for_messages(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let redis_client = Arc::new(get_redis_client().await.unwrap());
        println!("Connected to RabbitMQ");

        // Open a channel for the main process
        let channel = self.rabbit_mq_conn.open_channel(None)?;

        // Declare the queue
        let new_file_extract_queue = channel.queue_declare(
            "CSV_UPLOAD",
            QueueDeclareOptions {
                durable: true,
                ..QueueDeclareOptions::default()
            },
        )?;

        // Start consuming messages
        let file_extract_queue_consumer = new_file_extract_queue.consume(ConsumerOptions::default())?;
        println!("Waiting for messages...");

        let semaphore = Arc::new(Semaphore::new(5));

        for message in file_extract_queue_consumer.receiver().iter() {
            match message {
                ConsumerMessage::Delivery(delivery) => {
                    let message: Result<NewFileProcessQueue, &str> = Self::get_message(&delivery);
                    if message.is_ok() {
                        let message = message.unwrap();
                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        task::spawn(async move {
                            if let Err(processing_error) = process_woocommerce_csv(
                                message.clone().file,
                                std::env::var("WOOCOMMERCE_URL").unwrap(),
                                std::env::var("WOOCOMMERCE_CONSUMER_KEY").unwrap(),
                                std::env::var("WOOCOMMERCE_CONSUMER_SECRET").unwrap(),
                            )
                            .await {
                                println!("Error processing file: {}", processing_error);
                            } else {
                                println!("File processed successfully: {}", message.file);
                            }
                            drop(permit);
                        });
                    }
                    file_extract_queue_consumer.ack(delivery)?;
                }
                other => {
                    println!("Consumer ended: {:?}", other);
                    break;
                }
            }
        }

        self.close_conn()?;
        Ok(())
    }

    fn get_message(delivery: &Delivery) -> Result<NewFileProcessQueue, &'static str> {
        let msg = serde_json::from_slice::<NewFileProcessQueue>(&delivery.body)
            .map_err(|_| "Failed to parse message")?;
        Ok(msg)
    }



    fn close_conn(self) -> AmiqpResult<()> {
        self.rabbit_mq_conn.close()
    }

}