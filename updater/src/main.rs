mod worker; 
mod libs;
mod helper;
mod controllers;
// mod types;

#[tokio::main]
async fn main() {
    // controllers::woocommerce_processor::process_woocommerce_csv(file_path, base_url, consumer_key, consumer_secret)
    let _ = worker::run_worker().await;
}
