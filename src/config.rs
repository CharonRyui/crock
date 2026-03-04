use std::sync::OnceLock;

use serde::Deserialize;

use crate::{tasks::Task, utils::parse_time};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub tasks: Vec<ConfigTask>,
}

#[derive(Debug, Deserialize)]
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

pub fn get_config_tasks() -> &'static Vec<Task> {
    TASKS_IN_FILE.get_or_init(|| {
        let config_tasks = &CONFIG
            .get_or_init(|| {
                let content = std::fs::read_to_string("config/config.toml")
                    .expect("fail to read config file");
                toml::from_str(&content).expect("invalid config file format")
            })
            .tasks;
        config_tasks.iter().map(Task::from).collect()
    })
}
