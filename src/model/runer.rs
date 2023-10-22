use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Rune {
    pub blueprints: Option<HashMap<String, Blueprint>>,
    pub env: Option<HashMap<String, Vec<(String, String)>>>,
    pub flows: Option<Vec<Flow>>,
}

#[derive(Deserialize)]
pub struct Image {
    pub context: String,
    pub tag: String,
    pub options: Option<Vec<String>>,
    pub build_args: Option<Vec<String>>,
    pub pre: Option<Vec<(ExecutionEnvironment, String)>>,
    pub post: Option<Vec<(ExecutionEnvironment, String)>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ExecutionEnvironment {
    Local,
    Container(String),
}

#[derive(Deserialize)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub ports: Option<(String, String)>,
    pub env: Option<Vec<(String, String)>>,
    pub volumes: Option<Vec<(String, String)>>,
    pub entrypoint: Option<Vec<String>>,
    pub hc: Option<HealthCheck>,
}

#[derive(Deserialize)]
pub struct HealthCheck {
    pub command: (ExecutionEnvironment, String),
    pub interval: Option<String>,
    pub retries: Option<u32>,
}

#[derive(Deserialize)]
pub struct Shell {
    pub commands: Vec<String>,
    pub env: Option<Vec<(String, String)>>,
}

#[derive(Deserialize)]
pub struct Blueprint {
    pub _env: Option<Vec<(String, String)>>,
    pub image: Option<Image>,
    pub container: Option<Container>,
    pub shell: Option<Shell>,
}

#[derive(Deserialize, Clone)]
pub struct Flow {
    pub name: String,
    pub tasks: Vec<Task>,
    pub pkg_dependencies: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
pub struct Task {
    pub id: u32,
    #[serde(rename = "type")]
    pub typ: TaskType,
    pub name: String,
    pub job: JobType,
    pub depends: Option<u32>,
}

#[derive(Deserialize, Clone)]
pub enum TaskType {
    Blueprint,
    Env,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Container,
    Image,
    Shell,
    Set,
}
