use config::{Config, File, FileFormat};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[derive(Deserialize)]
struct Conf {
    api: Api,
    db: Db,
    env: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct Api {
    env: ApiEnv,
    image: Image,
}

#[derive(Deserialize)]
struct ApiEnv {
    host: String,
    port: u32,
    db_pool_max_connections: u32,
}

#[derive(Deserialize)]
struct Image {
    context: String,
    tag: Option<String>,
    options: Option<Vec<String>>,
    build_args: Option<Vec<String>>,
    pre: Option<Vec<Vec<String>>>,
    post: Option<Vec<Vec<String>>>,
}

#[derive(Deserialize)]
struct Db {
    user: String,
    password: String,
    name: String,
    host: String,
    port: u32,
}

fn main() {
    let conf: Conf = Config::builder()
        .add_source(File::new("build.yaml", FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize::<Conf>()
        .unwrap();
    check_package_dependencies(vec!["psql", "sqlx"]);
    wait_for_postgres_to_get_ready(&conf.db);
    set_environment_variables(conf.env);
    create_docker_image(DockerBuild {
        context: &PathBuf::from(conf.api.image.context),
        image_tag: conf.api.image.tag,
        cmd_options: conf.api.image.options,
        build_args: conf.api.image.build_args,
        pre: conf.api.image.pre,
        post: conf.api.image.post,
    });
}

/// Package Dependency Check:
/// Checks whether the execution environment has the necessary packages given
/// installed.
///
/// ---
/// Panics if it finds an uninstalled package.
fn check_package_dependencies(deps: Vec<&str>) {
    deps.iter().for_each(|d| {
        let d_exists = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", d))
            .output()
            .expect("Failed to execute <command>");
        if !d_exists.status.success() {
            panic!("{} is not installed", d);
        }
    });
}

/// Postgres Connection Readiness Check:
/// Checks whether the Postgres instance ready for operation. It runs a simple
/// <psql> command for checking. Retries the command every second if it fails.
fn wait_for_postgres_to_get_ready(db_conf: &Db) {
    let connection_string = format!(
        "postgres://{user}:{password}@{host}:{port}/{name}",
        user = db_conf.user,
        password = db_conf.password,
        host = db_conf.host,
        port = db_conf.port,
        name = db_conf.name
    );
    let mut check = Command::new("psql");
    check.arg(&connection_string).args(["-c", "\\q"]);

    while !check
        .output()
        .map_err(|_| eprintln!("Failed to execute <psql>"))
        .unwrap()
        .status
        .success()
    {
        eprintln!("DB is not ready");
        sleep(Duration::from_secs(1));
    }
}

struct DockerBuild<'a> {
    context: &'a Path,
    image_tag: Option<String>,
    cmd_options: Option<Vec<String>>,
    build_args: Option<Vec<String>>,
    pre: Option<Vec<Vec<String>>>,
    post: Option<Vec<Vec<String>>>,
}

fn create_docker_image(docker_build: DockerBuild) {
    if let Some(pre) = docker_build.pre {
        let mut cmds = create_commands_from_tokens(pre);
        cmds.iter_mut().for_each(|cmd| {
            let result = cmd
                .current_dir(docker_build.context)
                .output()
                .expect("Failed to execute command");
            if !result.status.success() {
                panic!("RESULT: {:?}", result)
            }
        });
    }

    // TODO: implement <docker build> command
    let mut docker_build_command = Command::new("docker");

    docker_build_command.arg("build");

    if let Some(cmd_options) = docker_build.cmd_options {
        cmd_options.iter().for_each(|cmd_option| {
            docker_build_command.arg(cmd_option);
        })
    }

    if let Some(image_tag) = docker_build.image_tag {
        docker_build_command.args(["-t", &image_tag]);
    }

    if let Some(build_args) = docker_build.build_args {
        build_args.iter().for_each(|build_arg| {
            docker_build_command.arg(format!("--build-arg=\"{}\"", build_arg));
        });
    }

    docker_build_command.arg(docker_build.context);

    let result = docker_build_command
        .output()
        .expect("Failed to execute docker build command.");
    if !result.status.success() {
        panic!("RESULT: {:?}", result)
    }

    if let Some(post) = docker_build.post {
        let mut cmds = create_commands_from_tokens(post);
        cmds.iter_mut().for_each(|cmd| {
            let result = cmd
                .current_dir(docker_build.context)
                .output()
                .expect("Failed to execute command");
            if !result.status.success() {
                panic!("RESULT: {:?}", result)
            }
        });
    }
}

fn create_commands_from_tokens<'a>(tokens_vec: Vec<Vec<String>>) -> Vec<Command> {
    tokens_vec
        .iter()
        .map(|tokens| {
            if !tokens.is_empty() {
                let mut cmd = Command::new(&tokens[0]);
                tokens[1..].iter().for_each(|token| {
                    cmd.arg(token);
                });
                return cmd;
            } else {
                panic!("Empty command tokens array is not allowed.")
            }
        })
        .collect()
}

fn set_environment_variables(key_values: Vec<(String, String)>) {
    key_values.iter().for_each(|p| {
        std::env::set_var(&p.0, &p.1);
    })
}
