use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};

use anyhow::{anyhow, Result};
use log::info;

use crate::run_task;

use super::state::State;

pub fn execute_flow(flow_idx: usize, state: State) -> Result<()> {
    if let Some(flows) = state.flows {
        if let Some(flow) = flows.get(flow_idx) {
            if let Some(pkg_dependencies) = &flow.pkg_dependencies {
                check_package_dependencies(&pkg_dependencies);
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
fn check_package_dependencies(deps: &Vec<String>) {
    println!("Checking package dependencies");
    deps.iter().for_each(|d| {
        let d_exists = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", d))
            .stdout(Stdio::null())
            .output()
            .expect("Failed to execute <command>");
        if !d_exists.status.success() {
            panic!("{} is not installed", d);
        }
    });
}
