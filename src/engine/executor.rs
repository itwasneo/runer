//use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{error, info};
use smol::process::{Child, Command, Stdio};

use crate::model::runer::Shell;
use crate::run_task;

use super::state::State;

pub async fn execute_flow(flow_idx: usize, state: State) -> Result<()> {
    if let Some(flows) = state.flows {
        if let Some(flow) = flows.get(flow_idx) {
            if let Some(pkg_dependencies) = &flow.pkg_dependencies {
                check_package_dependencies(&pkg_dependencies).await?;
            }

            let mut thread_handlers: Vec<JoinHandle<()>> = vec![];

            if !flow.tasks.is_empty() {
                flow.tasks.iter().for_each(|task| {
                    let task = task.clone();
                    let handles = state.handles.as_ref().unwrap().clone();
                    let blueprints = state.blueprints.as_ref().unwrap().clone();
                    let env = state.env.as_ref().unwrap().clone();
                    if let Some(_) = task.depends {
                        let thread_handler = thread::spawn(move || {
                            run_task(task, handles, blueprints, env);
                        });
                        thread_handlers.push(thread_handler);
                    } else {
                        run_task(task, handles, blueprints, env);
                    }
                });

                for h in thread_handlers {
                    h.join().unwrap();
                }
                // Making sure every command exited gracefully
                if let Ok(mut handles) = state.handles.as_ref().unwrap().lock() {
                    handles.iter_mut().for_each(|p| {
                        // TODO: Change this to try_wait and handle results separately
                        match p.1.try_wait() {
                            Ok(Some(status)) => info!("Task {} exited with: {status}", p.0),
                            Ok(None) => {
                                let _ = p.1.wait();
                            }
                            Err(e) => panic!("error attempting to wait: {e}"),
                        }
                    });
                }
            } else {
                return Err(anyhow!("Flow should have at least one task."));
            }
        } else {
            return Err(anyhow!("No flow found for given idx."));
        }
    } else {
        return Err(anyhow!("Application state has no flow to run."));
    }
    Ok(())
}

/// Package Dependency Check:
/// Checks whether the execution environment has the necessary packages given
/// installed.
///
/// ---
/// Panics if it finds an uninstalled package.
pub async fn check_package_dependencies(deps: &Vec<String>) -> Result<()> {
    info!("Checking package dependencies");
    let (tx, rx) = std::sync::mpsc::channel::<Child>();
    for d in deps {
        let d = d.clone();
        tx.send(
            Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {}", d))
                .stdout(Stdio::null())
                .spawn()?,
        )
        .unwrap();
    }
    for _ in deps {
        match rx.recv() {
            Ok(mut child) => {
                if !child.status().await?.success() {
                    return Err(anyhow!("Missing dependency"));
                }
            }
            Err(_) => return Err(anyhow!("CHANNEL ERROR")),
        }
    }
    Ok(())
}

// TODO: Instead of setting environment_variables with private function
// use .env method of Command struct.
pub fn run_shell_script(shell: &Shell) -> Result<Child, std::io::Error> {
    info!("Starting to run shell script");
    if let Some(environment_variables) = &shell.env {
        set_environment_variables(environment_variables);
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
pub fn set_environment_variables(key_values: &Vec<(String, String)>) {
    key_values.iter().for_each(|p| {
        std::env::set_var(&p.0, &p.1);
    });
}
