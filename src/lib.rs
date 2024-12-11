use std::time::Duration;

use log::{LevelFilter, Log};
use rdkafka::{
  producer::{BaseProducer, BaseRecord, Producer},
  ClientConfig,
};
use serde::{Deserialize, Serialize};

/// Same as init but uses KAFKA_BROKER, KAFKA_TOPIC_LOG and RUST_LOG environment variables as parameters
pub fn init_from_env() {
  let server = std::env::var("KAFKA_BROKER").expect("Failed to load KAFKA_BROKER");
  let topic = std::env::var("KAFKA_TOPIC_LOG").expect("Failed to load KAFKA_TOPIC_LOG");
  let filter = std::env::var("RUST_LOG")
    .map(|level| level.parse().unwrap_or(LevelFilter::Info))
    .unwrap_or(LevelFilter::Info);

  init(&server, filter, &topic);
}

/// Init the kafka logger
///
/// # Examples
/// ```
/// kafka_logger::init("localhost:9093", log::LevelFilter::Debug, "logging");
///
/// log::info!("test");
///
/// kafka_logger::shutdown();  
/// ```
pub fn init(server: &str, filter: LevelFilter, topic: &str) {
  log::set_boxed_logger(Box::new(Logger::init(server, filter, topic)))
    .expect("Failed to set Logger");
  log::set_max_level(filter);
}

/// Waits for all remaining log messages to be send to kafka
pub fn shutdown() {
  log::logger().flush();
}

/// Format of log messages send to kafka
#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
  pub level: String,
  pub target: String,
  pub message: String,
}

struct Logger {
  producer: BaseProducer,
  filter: LevelFilter,
  topic: String,
}

impl Logger {
  fn init(server: &str, filter: LevelFilter, topic: &str) -> Self {
    let producer: BaseProducer = ClientConfig::new()
      .set("bootstrap.servers", server)
      .set("message.timeout.ms", "5000")
      .create()
      .expect("Failed to create logger");

    Self {
      producer,
      filter,
      topic: topic.into(),
    }
  }
}

impl Drop for Logger {
  fn drop(&mut self) {
    self.flush();
  }
}

impl Log for Logger {
  fn enabled(&self, metadata: &log::Metadata) -> bool {
    self
      .filter
      .to_level()
      .map(|level| metadata.level() <= level)
      .unwrap_or(false)
  }

  fn flush(&self) {
    self
      .producer
      .flush(Duration::from_secs(2))
      .expect("Failed to flush");
  }

  fn log(&self, record: &log::Record) {
    if self.enabled(record.metadata()) && !record.target().starts_with("rdkafka") {
      let _ = self.producer.send(
        BaseRecord::to(&self.topic)
          .payload(
            &serde_json::to_string(&Record {
              level: record.level().to_string(),
              target: record.target().to_string(),
              message: record.args().to_string(),
            })
            .unwrap(),
          )
          .key(&()),
      );
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{init_from_env, shutdown};

  #[test]
  fn test_env_init() {
    std::env::set_var("KAFKA_BROKER", "localhost");
    std::env::set_var("KAFKA_TOPIC_LOG", "logging");

    init_from_env();
    shutdown();
  }
}
