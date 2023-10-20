use config::{Config, File, FileFormat};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

#[derive(Deserialize)]
struct Conf {
    blueprints: Option<Vec<Blueprint>>,
    env: Vec<(String, String)>,
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
struct Blueprint {
    name: String,
    env: Option<Vec<(String, String)>>,
    image: Option<Image>,
}

fn main() {
    let conf: Conf = Config::builder()
        .add_source(File::new(".runer", FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize::<Conf>()
        .unwrap();
    check_package_dependencies(vec!["psql", "sqlx"]);

    if let Some(plans) = conf.blueprints {
        // TODO: Change this ==================================================
        if let Some(db_plan) = plans.iter().find(|p| p.name.eq("db")) {
            if let Some(env) = &db_plan.env {
                let user = env
                    .iter()
                    .find(|p| p.0.eq("user"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<user> env variable is needed."));
                let password = env
                    .iter()
                    .find(|p| p.0.eq("password"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<password> env variable is needed."));
                let name = env
                    .iter()
                    .find(|p| p.0.eq("name"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<name> env variable is needed."));
                let host = env
                    .iter()
                    .find(|p| p.0.eq("host"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<host> env variable is needed."));
                let port = env
                    .iter()
                    .find(|p| p.0.eq("port"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<port> env variable is needed."));
                wait_for_postgres_to_get_ready(user, password, host, port, name);
            }
        }
        // ====================================================================

        set_environment_variables(conf.env);

        // TODO: Change this ==================================================
        if let Some(api) = plans.iter().find(|p| p.name.eq("api")) {
            if let Some(image) = &api.image {
                create_docker_image(DockerBuild {
                    context: &PathBuf::from(&image.context),
                    image_tag: &image.tag,
                    cmd_options: &image.options,
                    build_args: &image.build_args,
                    pre: &image.pre,
                    post: &image.post,
                });
            }
        }
        // ====================================================================
    }
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
fn wait_for_postgres_to_get_ready(user: &str, password: &str, host: &str, port: &str, name: &str) {
    let connection_string = format!(
        "postgres://{user}:{password}@{host}:{port}/{name}",
        user = user,
        password = password,
        host = host,
        port = port,
        name = name
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
    image_tag: &'a Option<String>,
    cmd_options: &'a Option<Vec<String>>,
    build_args: &'a Option<Vec<String>>,
    pre: &'a Option<Vec<Vec<String>>>,
    post: &'a Option<Vec<Vec<String>>>,
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

fn create_commands_from_tokens<'a>(tokens_vec: &Vec<Vec<String>>) -> Vec<Command> {
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
