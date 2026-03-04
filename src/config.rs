use std::{env, path::PathBuf, sync::OnceLock};

use serde::Deserialize;
use tracing::{info, instrument};

use crate::{tasks::Task, utils::parse_time};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub tasks: Vec<ConfigTask>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ConfigTask {
    pub desc: String,
    pub time: String,
}

static CONFIG: OnceLock<Config> = OnceLock::new();
static TASKS_IN_FILE: OnceLock<Vec<Task>> = OnceLock::new();

impl From<&ConfigTask> for Task {
    fn from(value: &ConfigTask) -> Self {
        Self {
            content: value.desc.clone().into(),
            seconds: parse_time(&value.time),
        }
    }
}

#[instrument]
pub fn get_config_tasks() -> &'static Vec<Task> {
    TASKS_IN_FILE.get_or_init(|| {
        if let Ok(home_path) = env::var("HOME") {
            let mut path_buf: PathBuf = home_path.into();
            path_buf.push(".config/crock/config.toml");
            info!("reading config file in {}", path_buf.display());
            let config_tasks = &CONFIG
                .get_or_init(|| {
                    let content = std::fs::read_to_string(&path_buf).unwrap_or_default();
                    toml::from_str(&content).unwrap_or_default()
                })
                .tasks;
            config_tasks.iter().map(Task::from).collect()
        } else {
            Vec::new()
        }
    })
}
