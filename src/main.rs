use config::{Config, File, FileFormat};
use serde::Deserialize;
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
    tag: String,
    options: Option<Vec<String>>,
    build_args: Option<Vec<String>>,
    pre: Option<Vec<(ExecutionEnvironment, Vec<String>)>>,
    post: Option<Vec<(ExecutionEnvironment, Vec<String>)>>,
}

#[derive(Deserialize)]
enum ExecutionEnvironment {
    Local,
    Container(String),
}

#[derive(Deserialize)]
struct Container {
    name: String,
    image: String,
    ports: Option<(String, String)>,
    env: Option<Vec<(String, String)>>,
    volumes: Option<Vec<(String, String)>>,
    entrypoint: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Blueprint {
    name: String,
    env: Option<Vec<(String, String)>>,
    image: Option<Image>,
    container: Option<Container>,
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
        if let Some(db) = plans.iter().find(|p| p.name.eq("db")) {
            if let Some(container) = &db.container {
                run_docker_container(container);
            }
        }

        if let Some(db_plan) = plans.iter().find(|p| p.name.eq("db")) {
            if let Some(env) = &db_plan.env {
                let user = env
                    .iter()
                    .find(|p| p.0.eq("POSTGRES_USER"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<user> env variable is needed."));
                let password = env
                    .iter()
                    .find(|p| p.0.eq("POSTGRES_PASSWORD"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<password> env variable is needed."));
                let name = env
                    .iter()
                    .find(|p| p.0.eq("POSTGRES_DB"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<name> env variable is needed."));
                let host = env
                    .iter()
                    .find(|p| p.0.eq("POSTGRES_HOST"))
                    .and_then(|p| Some(&p.1))
                    .unwrap_or_else(|| panic!("<host> env variable is needed."));
                let port = env
                    .iter()
                    .find(|p| p.0.eq("POSTGRES_PORT"))
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
                create_docker_image(image);
            }

            if let Some(container) = &api.container {
                run_docker_container(container);
            }
        }

        if let Some(crawler) = plans.iter().find(|p| p.name.eq("crawler")) {
            if let Some(image) = &crawler.image {
                create_docker_image(image);
            }

            if let Some(container) = &crawler.container {
                run_docker_container(container);
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

/// Creates a new docker image(if it doesn't exist) according to given Image.
///
/// ---
/// Panics if <docker build> command returns non-success code.
fn create_docker_image(docker_image: &Image) {
    println!("Creating docker image for {}", docker_image.tag);
    if let Some(pre) = &docker_image.pre {
        let mut cmds = create_commands_from_tokens(pre);
        cmds.iter_mut().for_each(|cmd| {
            let result = cmd
                .current_dir(&docker_image.context)
                .output()
                .expect("Failed to execute command");
            if !result.status.success() {
                panic!("RESULT: {:?}", result)
            }
        });
    }

    let mut docker_build_command = Command::new("docker");
    docker_build_command.arg("build");

    if let Some(cmd_options) = &docker_image.options {
        cmd_options.iter().for_each(|cmd_option| {
            docker_build_command.arg(cmd_option);
        })
    }

    docker_build_command.args(["-t", &docker_image.tag]);

    if let Some(build_args) = &docker_image.build_args {
        build_args.iter().for_each(|build_arg| {
            docker_build_command.arg(format!("--build-arg=\"{}\"", build_arg));
        });
    }

    docker_build_command.arg(&docker_image.context);

    let result = docker_build_command
        .output()
        .expect("Failed to execute docker build command.");
    if !result.status.success() {
        panic!("RESULT: {:?}", result)
    }
    println!("Docker image is created for {}", docker_image.tag);

    if let Some(post) = &docker_image.post {
        let mut cmds = create_commands_from_tokens(&post);
        cmds.iter_mut().for_each(|cmd| {
            let result = cmd
                .current_dir(&docker_image.context)
                .output()
                .expect("Failed to execute command");
            if !result.status.success() {
                panic!("RESULT: {:?}", result)
            }
        });
    }
}

/// Runs a new docker container according to the given Container.
///
/// ---
/// Panics if an empty <entrypoint> command token array is provided.
/// Panics if <docker run> command returns non-success code.
fn run_docker_container(docker_container: &Container) {
    let mut docker_run_command = Command::new("docker");
    docker_run_command.arg("run");

    docker_run_command.arg("-d");

    docker_run_command.args(["--name", &docker_container.name]);

    if let Some(env) = &docker_container.env {
        env.iter().for_each(|p| {
            // docker_run_command.arg(format!("--env {}={}", p.0, p.1));
            docker_run_command.args(["--env", &format!("{}={}", p.0, p.1)]);
        })
    }

    if let Some(ports) = &docker_container.ports {
        docker_run_command.args(["-p", &format!("{}:{}", ports.0, ports.1)]);
    }

    if let Some(volumes) = &docker_container.volumes {
        volumes.iter().for_each(|v| {
            docker_run_command.args(["-v", &format!("{}:{}", v.0, v.1)]);
        })
    }

    if let Some(entrypoint) = &docker_container.entrypoint {
        if !entrypoint.is_empty() {
            docker_run_command.args(["--entrypoint", &entrypoint[0]]);
            entrypoint[1..].iter().for_each(|t| {
                docker_run_command.arg(&t);
            })
        } else {
            panic!("Missing entrypoint command/arguments.");
        }
    }

    // TODO: Change this
    docker_run_command.arg("--net=last_default");

    docker_run_command.arg(&docker_container.image);

    let result = docker_run_command
        .output()
        .expect("Failed to execute docker build command.");
    if !result.status.success() {
        // TODO: In this case gracfully shutdown instead of panic.
        panic!("RESULT: {:?}", result)
    }

    println!("Running container: {}", docker_container.name);
}

/// Creates a Command list from the given list of (ExecutionEnvironment, Vec<String>)
/// tuples.
///
/// It uses the first element of the Vec as the <command>, the rest is evaluated
/// as <args>.
///
/// Depending on the given ExecutionEnvironment, it can modify the <command> to
/// be run in the given container.
///
/// ---
/// Panics if the given Vec<String> is empty.
fn create_commands_from_tokens<'a>(
    env_and_tokens: &Vec<(ExecutionEnvironment, Vec<String>)>,
) -> Vec<Command> {
    env_and_tokens
        .iter()
        .map(|e_ts| {
            if !e_ts.1.is_empty() {
                match &e_ts.0 {
                    ExecutionEnvironment::Local => {
                        let mut cmd = Command::new(&e_ts.1[0]);
                        e_ts.1[1..].iter().for_each(|token| {
                            cmd.arg(token);
                        });
                        return cmd;
                    }
                    ExecutionEnvironment::Container(_container_identification) => {
                        todo!("Implement command execution in Container")
                    }
                }
            } else {
                panic!("Empty command tokens array is not allowed.")
            }
        })
        .collect()
}

/// Sets the given (String, String) tuples as environment variables inside the
/// **execution** environment.
///
/// First element gets used as KEY. Second element gets used as VALUE.
fn set_environment_variables(key_values: Vec<(String, String)>) {
    key_values.iter().for_each(|p| {
        std::env::set_var(&p.0, &p.1);
    })
}
