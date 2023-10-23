mod engine;
mod model;

use anyhow::Result;
use engine::extractor::*;
use log::info;
use model::runer::*;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

use self::engine::executor::{run_shell_script, set_environment_variables};

fn main() -> Result<()> {
    let start = Instant::now();
    env_logger::init();

    let rune = extract_rune(".runer")?;

    analyze_fragments(&rune);

    let state = State::default().from_rune(rune);
    smol::block_on(execute_flow(0, state))?;

    let duration = start.elapsed();
    info!("Time elapsed: {:?}", duration);
    Ok(())
}

pub fn wait_until_parent_task_command_is_finished(
    parent_handle_id: u32,
    child_task_id: u32,
    handles: Arc<Mutex<HashMap<u32, Child>>>,
) {
    // Here it waits until the parent handle becomes available ================
    let mut available = false;
    while !available {
        if let Ok(handles) = handles.lock() {
            if handles.contains_key(&parent_handle_id) {
                available = true;
            }
        }
        thread::sleep(Duration::from_millis(5));
    }

    // Then it makes sure that the parent task finishes successfully ==========
    let mut finished_successfully = false;
    while !finished_successfully {
        if let Ok(mut handles) = handles.lock() {
            let handle = handles.get_mut(&parent_handle_id).unwrap_or_else(|| {
                panic!(
                    "Task {} can not find the parent process handle",
                    child_task_id
                )
            });
            match handle.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        finished_successfully = true;
                    }
                }
                Ok(None) => {}
                Err(e) => panic!("error attempting to wait: {e}"),
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub fn run_task(
    t: Task,
    handles: Arc<Mutex<HashMap<u32, Child>>>,
    blueprints: Arc<HashMap<String, Blueprint>>,
    local_global_env: Arc<HashMap<String, Vec<(String, String)>>>,
) {
    if let Some(depends) = t.depends {
        wait_until_parent_task_command_is_finished(depends, t.id, handles.clone());
    }
    if let Ok(mut handles) = handles.lock() {
        match t.typ {
            TaskType::Blueprint => {
                let (tag, blueprint) = blueprints.get_key_value(&t.name).unwrap_or_else(|| {
                    panic!("For {}, no blueprint found.", t.name);
                });
                match t.job {
                    JobType::Image => {
                        let handle =
                            create_docker_image(blueprint.image.as_ref().unwrap_or_else(|| {
                                panic!("No image job is defined in {}'s blueprint", tag);
                            }));
                        handles.insert(t.id, handle);
                    }
                    JobType::Container => {
                        let handle = run_docker_container(
                            blueprint.container.as_ref().unwrap_or_else(|| {
                                panic!("No container job is defined {}'s blueprint", tag);
                            }),
                        );
                        handles.insert(t.id, handle);
                    }
                    JobType::Set => {
                        todo!("Decide how to handle 'Set' jobs inside blueprints")
                    }
                    JobType::Shell => {
                        let _handle =
                            run_shell_script(blueprint.shell.as_ref().unwrap_or_else(|| {
                                panic!("No shell job is defined {}'s blueprint", tag);
                            }));
                        // handles.insert(t.id, handle);
                    }
                }
            }
            TaskType::Env => {
                let (_tag, env) = local_global_env.get_key_value(&t.name).unwrap_or_else(|| {
                    panic!("For {}, no environment variable is found.", t.name);
                });
                let _handle = set_environment_variables(env);
                // handles.insert(t.id, handle);
            }
        }
    }
}

/// Creates a new docker image(if it doesn't exist) according to given Image.
///
/// ---
/// Panics if <docker build> command returns non-success code.
pub fn create_docker_image(docker_image: &Image) -> Child {
    println!("Starting to create docker image for {}", docker_image.tag);
    if let Some(pre) = &docker_image.pre {
        pre.iter().for_each(|p| {
            Command::new("sh")
                .arg("-c")
                .arg(p.1.clone())
                .output()
                .expect("Something went wrong");
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

    docker_build_command
        .arg(&docker_image.context)
        .output()
        .expect("Something went wrong");

    if let Some(post) = &docker_image.post {
        post.iter().for_each(|p| {
            Command::new("sh")
                .arg("-c")
                .arg(p.1.clone())
                .output()
                .expect("Something went wrong");
        });
    }

    Command::new("sh")
        .current_dir(&docker_image.context)
        .arg("-c")
        .arg("echo DONE")
        .stdout(Stdio::null())
        .spawn()
        .expect("Spawning <docker build> command failed.")
}

/// Runs a new docker container according to the given Container.
///
/// ---
/// Panics if an empty <entrypoint> command token array is provided.
/// Panics if an empty <healthcheck> command token array is provided.
/// Panics if <docker run> command returns non-success code.
pub fn run_docker_container(docker_container: &Container) -> Child {
    println!("Starting {}", docker_container.name);
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

    docker_run_command
        .stdout(Stdio::null())
        .spawn()
        .expect("Spawning <docker run> command failed.")
}
