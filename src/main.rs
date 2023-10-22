mod engine;
mod model;

use anyhow::Result;
use engine::extractor::*;
use model::runer::*;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::engine::executor::execute_flow;
use crate::engine::state::State;

fn main() -> Result<()> {
    let start = Instant::now();
    env_logger::init();

    let rune = extract_rune(".runer")?;

    analyze_fragments(&rune);

    let state = State::default().from_rune(rune);
    // run_flows(state);
    execute_flow(0, state).unwrap();
    // ========================================================================
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
    Ok(())
}

// pub fn run_flows(state: State) {
//     // let flows = state.flows.as_ref().unwrap();
//     if let Some(flows) = state.flows {
//         let flows = flows.as_ref().clone();
//         flows.iter().for_each(|f| {
//             if let Some(pkg_dependencies) = &f.pkg_dependencies {
//                 check_package_dependencies(pkg_dependencies);
//             }
//
//             let mut thread_handlers: Vec<JoinHandle<()>> = vec![];
//
//             if !f.tasks.is_empty() {
//                 // f.tasks.iter().for_each(|t| {
//                 //     let handles = state.handles.as_ref().unwrap().clone();
//                 //     let blueprints = state.blueprints.as_ref().unwrap().clone();
//                 //     let env = state.env.as_ref().unwrap().clone();
//                 //     if let Some(_) = t.depends {
//                 //         let thread_handler = thread::spawn(|| {
//                 //             run_task(t.clone(), handles, blueprints, env);
//                 //         });
//                 //         thread_handlers.push(thread_handler);
//                 //     } else {
//                 //         run_task(t.clone(), handles, blueprints, env);
//                 //     }
//                 // });
//                 for t in f.tasks {
//                     let handles = state.handles.as_ref().unwrap().clone();
//                     let blueprints = state.blueprints.as_ref().unwrap().clone();
//                     let env = state.env.as_ref().unwrap().clone();
//                     if let Some(_) = t.depends {
//                         let thread_handler = thread::spawn(move || {
//                             run_task(t, handles, blueprints, env);
//                         });
//                         thread_handlers.push(thread_handler);
//                     } else {
//                         run_task(t, handles, blueprints, env);
//                     }
//                 }
//
//                 for h in thread_handlers {
//                     h.join().unwrap();
//                 }
//
//                 // Making sure every command exited gracefully
//                 if let Ok(mut handles) = state.handles.as_ref().unwrap().lock() {
//                     handles.iter_mut().for_each(|p| {
//                         // TODO: Change this to try_wait and handle results separately
//                         match p.1.try_wait() {
//                             Ok(Some(status)) => println!("exited with: {status}"),
//                             Ok(None) => {
//                                 println!("status not ready yet, let's really wait");
//                                 let res = p.1.wait();
//                                 println!("result: {res:?}");
//                             }
//                             Err(e) => panic!("error attempting to wait: {e}"),
//                         }
//                     });
//                 }
//             } else {
//                 panic!("Flow has to have at least one task.");
//             }
//         })
//     }
// }

fn wait_until_parent_task_command_is_finished(
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

fn run_task(
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
                        let handle =
                            run_shell_script(blueprint.shell.as_ref().unwrap_or_else(|| {
                                panic!("No shell job is defined {}'s blueprint", tag);
                            }));
                        handles.insert(t.id, handle);
                    }
                }
            }
            TaskType::Env => {
                let (_tag, env) = local_global_env.get_key_value(&t.name).unwrap_or_else(|| {
                    panic!("For {}, no environment variable is found.", t.name);
                });
                let handle = set_environment_variables(env);
                handles.insert(t.id, handle);
            }
        }
    }
}

/// Creates a new docker image(if it doesn't exist) according to given Image.
///
/// ---
/// Panics if <docker build> command returns non-success code.
fn create_docker_image(docker_image: &Image) -> Child {
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
fn run_docker_container(docker_container: &Container) -> Child {
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

fn run_shell_script(shell: &Shell) -> Child {
    println!("Starting to run shell script");
    if let Some(environment_variables) = &shell.env {
        set_environment_variables(environment_variables)
            .wait()
            .expect("Something went wrong setting up environment variables");
    }
    Command::new("sh")
        .arg("-c")
        .arg(shell.commands.join(" && "))
        .spawn()
        .expect("Spawning <script> command failed.")
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
// fn create_commands_from_tokens<'a>(
//     env_and_tokens: &Vec<(ExecutionEnvironment, String)>,
// ) -> Vec<Command> {
//     env_and_tokens
//         .iter()
//         .map(|e_ts| {
//             if !e_ts.1.is_empty() {
//                 match &e_ts.0 {
//                     ExecutionEnvironment::Local => {
//                         let mut cmd = Command::new(&e_ts.1[0]);
//                         e_ts.1[1..].iter().for_each(|token| {
//                             cmd.arg(token);
//                         });
//                         return cmd;
//                     }
//                     ExecutionEnvironment::Container(_container_identification) => {
//                         todo!("Implement command execution in Container")
//                     }
//                 }
//             } else {
//                 panic!("Empty command tokens array is not allowed.")
//             }
//         })
//         .collect()
// }

/// Sets the given (String, String) tuples as environment variables inside the
/// **execution** environment.
///
/// First element gets used as KEY. Second element gets used as VALUE.
fn set_environment_variables(key_values: &Vec<(String, String)>) -> Child {
    key_values.iter().for_each(|p| {
        std::env::set_var(&p.0, &p.1);
    });
    Command::new("sh")
        .arg("-c")
        .arg("echo \"Environment variables are set\"")
        .stdout(Stdio::null())
        .spawn()
        .expect("Spawning set_environment_variables success message command failed.")
}
