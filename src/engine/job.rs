use anyhow::Result;
use log::info;
use smol::process::{Child, Command, Stdio};

use crate::model::runer::{Container, ExecutionEnvironment, Image, Shell};

/// Creates a new docker image(if it doesn't exist) according to given Image.
///
/// ---
/// Panics if <docker build> command returns non-success code.
pub async fn create_docker_image(docker_image: &Image) -> Result<Child, std::io::Error> {
    info!("Starting to create docker image for {}", docker_image.tag);

    // Running <pre> commands synchronously
    if let Some(pre) = &docker_image.pre {
        for p in pre {
            Command::new("sh")
                .arg("-c")
                .arg(p.1.clone())
                .output()
                .await?;
        }
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

    // Running <docker build> command synchronously
    docker_build_command
        .arg(&docker_image.context)
        .output()
        .await?;

    // Running <post> commands synchronously
    if let Some(post) = &docker_image.post {
        for p in post {
            Command::new("sh")
                .arg("-c")
                .arg(p.1.clone())
                .output()
                .await?;
        }
    }

    Command::new("sh")
        .current_dir(&docker_image.context)
        .arg("-c")
        .arg(format!(
            "echo \"Image creation is done for {}\"",
            &docker_image.tag
        ))
        .stdout(Stdio::null())
        .spawn()
}

/// Runs a new docker container according to the given Container.
///
/// ---
/// Panics if an empty <entrypoint> command token array is provided.
/// Panics if an empty <healthcheck> command token array is provided.
/// Panics if <docker run> command returns non-success code.
pub fn run_docker_container(docker_container: &Container) -> Result<Child, std::io::Error> {
    info!("Starting {}", docker_container.name);
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

    if let Some(hc) = &docker_container.hc {
        if !hc.command.1.is_empty() {
            match &hc.command.0 {
                ExecutionEnvironment::Local => {
                    todo!("Implement command execution in Container")
                }
                ExecutionEnvironment::Container(_container_identification) => {
                    docker_run_command.args(["--health-cmd", &hc.command.1]);
                }
            }
        } else {
            panic!("Missing healthcheck command/arguments.")
        }

        if let Some(interval) = &hc.interval {
            docker_run_command.args(["--health-interval", interval]);
        }
        if let Some(retries) = hc.retries {
            docker_run_command.args(["--health-retries", &retries.to_string()]);
        }
    }

    // TODO: Change this
    docker_run_command.arg("--net=last_default");

    docker_run_command.arg(&docker_container.image);

    docker_run_command.stdout(Stdio::null()).spawn()
}

// TODO: Instead of setting environment_variables with private function
// use .env method of Command struct.
pub async fn run_shell_script(shell: &Shell) -> Result<Child, std::io::Error> {
    info!("Starting to run shell script");
    if let Some(environment_variables) = &shell.env {
        match set_environment_variables(environment_variables) {
            Ok(mut child) => {
                child.status().await?;
            }
            Err(e) => panic!(
                "FATAL ERROR: Something went wrong setting up environment variables {}",
                e
            ),
        }
    }
    Command::new("sh")
        .arg("-c")
        .arg(shell.commands.join(" && "))
        .spawn()
}

/// Sets the given (String, String) tuples as environment variables inside the
/// **execution** environment.
///
/// First element gets used as KEY. Second element gets used as VALUE.
pub fn set_environment_variables(
    key_values: &Vec<(String, String)>,
) -> Result<Child, std::io::Error> {
    info!("Setting environment variables");
    key_values.iter().for_each(|p| {
        std::env::set_var(&p.0, &p.1);
    });
    Command::new("echo")
        .arg("Environment variables are set")
        .stdout(Stdio::null())
        .spawn()
}
