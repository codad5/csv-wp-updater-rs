use redis::{AsyncCommands, Client, RedisResult};
use tokio::sync::OnceCell;
use tonic::client;
use std::env;

static FILE_PROCESSING_MANAGER: OnceCell<FileProcessingManager> = OnceCell::const_new();
static REDIS_CLIENT: OnceCell<Client> = OnceCell::const_new();

#[derive(Debug, PartialEq, Clone)]
pub enum Status {
    Pending,
    Done,
    Failed,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ModelStatus {
    Queued,
    Downloading,
    Completed,
    Failed,
}

impl Status {
    fn to_string(&self) -> String {
        match self {
            Status::Pending => "pending".to_string(),
            Status::Done => "done".to_string(),
            Status::Failed => "failed".to_string(),
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "done" => Status::Done,
            "failed" => Status::Failed,
            _ => Status::Pending,
        }
    }
}

impl ModelStatus {
    fn to_string(&self) -> String {
        match self {
            ModelStatus::Queued => "queued".to_string(),
            ModelStatus::Downloading => "downloading".to_string(),
            ModelStatus::Completed => "completed".to_string(),
            ModelStatus::Failed => "failed".to_string(),
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "queued" => ModelStatus::Queued,
            "downloading" => ModelStatus::Downloading,
            "completed" => ModelStatus::Completed,
            "failed" => ModelStatus::Failed,
            _ => ModelStatus::Queued,
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::fmt::Display for ModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub async fn get_redis_client() -> RedisResult<&'static Client> {
    REDIS_CLIENT.get_or_try_init(|| async {
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        Client::open(redis_url)
    }).await
}


#[derive(Debug, Clone)]
pub struct RedisProgressManager {
    client: &'static Client,
    prefix: String,
}

impl RedisProgressManager {
    pub async fn new(prefix: &str) -> RedisResult<Self> {
        let client = get_redis_client().await?;
        Ok(Self {
            client,
            prefix: prefix.to_string(),
        })
    }

    async fn set_status(&self, id: &str, status: impl ToString) -> RedisResult<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}:status:{}", self.prefix, id);
        println!("Redis ==> {} ==> status {}", id, status.to_string());
        con.set(key, status.to_string()).await
    }

    async fn get_status(&self, id: &str) -> RedisResult<Option<String>> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}:status:{}", self.prefix, id);
        con.get(&key).await
    }

    pub async fn set_progress(&self, id: &str, progress: f32) -> RedisResult<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}:progress:{}", self.prefix, id);
        con.set(key, progress).await
    }

    pub async fn get_progress(&self, id: &str) -> RedisResult<f32> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}:progress:{}", self.prefix, id);
        let progress: Option<f32> = con.get(&key).await?;
        Ok(progress.unwrap_or(0.0))
    }

    pub async fn set_with_ttl(&self, id: &str, status: impl ToString, ttl: u64) -> RedisResult<()> {
        let mut con = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}:status:{}", self.prefix, id);
        con.set_ex(key, status.to_string(), ttl).await
    }
}

pub struct FileProcessingManager {
    redis: RedisProgressManager,
}

impl FileProcessingManager {
    pub async fn instance() -> RedisResult<&'static FileProcessingManager> {
        FILE_PROCESSING_MANAGER.get_or_try_init(|| async {
            Ok(Self {
                redis: RedisProgressManager::new("processing").await?,
            })
        }).await
    }

    pub async fn is_file_in_process(file_id: &str) -> RedisResult<bool> {
        let instance = Self::instance().await?;
        let status = Self::get_file_status(file_id).await?;
        Ok(status == Status::Pending)
    }

    pub async fn get_file_status(file_id: &str) -> RedisResult<Status> {
        let instance = Self::instance().await?;
        let status = instance.redis.get_status(file_id).await?;
        Ok(status.map_or(Status::Pending, |s| Status::from_string(&s)))
    }

    pub async fn start_file_process(file_id: &str, ttl: u64) -> RedisResult<()> {
        Self::instance().await.unwrap().redis.set_with_ttl(file_id, Status::Pending, ttl).await
    }

    pub async fn mark_as_done(file_id: &str) -> RedisResult<()> {
        Self::instance().await.unwrap().redis.set_status(file_id, Status::Done).await
    }

    pub async fn mark_as_failed(file_id: &str) -> RedisResult<()> {
        Self::instance().await.unwrap().redis.set_status(file_id, Status::Failed).await
    }

    pub async fn mark_progress(file_id: &str, page: u32, total: u32) -> RedisResult<()> {
        let instance = Self::instance().await.unwrap();

        let progress: f32 = if total == 0 {
            0.0
        } else {
            (page as f32 / total as f32) * 100.0
        };

        instance.redis.set_progress(file_id, progress).await?;

        if (progress - 100.0).abs() < f32::EPSILON {
            FileProcessingManager::mark_as_done(file_id).await?;
        }

        Ok(())
    }


    // a function to increment the progress by 1
  pub async fn increment_progress(file_id: &str, total: u32) -> RedisResult<()> {
        let instance = Self::instance().await.unwrap();

        let progress = instance.redis.get_progress(file_id).await?;
        let total_f = total as f32;
        let progress_f = progress;

        let mut total_processed = (progress_f * total_f / 100.0).ceil() + 1.0;
        if total_processed > total_f {
            total_processed = total_f;
        }

        let progress_f32: f32 = if total == 0 {
            0.0
        } else {
            ((total_processed * 100.0) / total_f).min(100.0)
        };

        println!(
            "\x1b[31mCurrent progress: {:.2}%\x1b[0m\n\
            \x1b[32mTotal: {}\x1b[0m\n\
            \x1b[34mTotal processed: {:.2}\x1b[0m\n\
            \x1b[33mfile_id: {}\x1b[0m\n",
            progress_f32, total, total_processed, file_id
        );

        instance.redis.set_progress(file_id, progress_f32).await?;

        if (progress_f32 - 100.0).abs() < f32::EPSILON {
            FileProcessingManager::mark_as_done(file_id).await?;
        }

        Ok(())
    }





    pub async fn get_progress(file_id: &str) -> RedisResult<f32> {
        Self::instance().await.unwrap().redis.get_progress(file_id).await
    }
}

// Backward compatibility functions

pub async fn is_file_in_process(file_id: &str) -> RedisResult<bool> {
    FileProcessingManager::is_file_in_process(file_id).await
}

pub async fn is_process_done(file_id: &str) -> RedisResult<bool> {
    let status = FileProcessingManager::get_file_status(file_id).await?;
    Ok(status == Status::Done)
}

pub async fn get_file_status(file_id: &str) -> RedisResult<Status> {
    FileProcessingManager::get_file_status(file_id).await
}

pub async fn start_file_process(file_id: &str, ttl: u64) -> RedisResult<()> {
    FileProcessingManager::start_file_process(file_id, ttl).await
}

pub async fn mark_as_done(file_id: &str) -> RedisResult<()> {
    FileProcessingManager::mark_as_done(file_id).await
}

pub async fn mark_as_failed(file_id: &str) -> RedisResult<()> {
    FileProcessingManager::mark_as_failed(file_id).await
}

pub async fn mark_progress(file_id: &str, page: u32, total: u32) -> RedisResult<()> {
    FileProcessingManager::mark_progress(file_id, page, total).await
}
