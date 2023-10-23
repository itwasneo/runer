use anyhow::{anyhow, Result};
use log::info;
use smol::channel;
use smol::process::{Child, Command, Stdio};

use super::state::State;
use super::task::run_task;

/// Executes a single Flow residing in the Application State.
/// Right now it takes an index and with it, it checks the Vec<Flow>.
pub async fn execute_flow(flow_idx: usize, state: State) -> Result<()> {
    if let Some(flows) = state.flows {
        if let Some(flow) = flows.get(flow_idx) {
            // Package dependencies are prioritized first in a Flow.
            // If the application fails to find a dependency throws an error.
            // Flow shouldn't continue if there is a missing dependency.
            if let Some(pkg_dependencies) = &flow.pkg_dependencies {
                check_package_dependencies(&pkg_dependencies).await?;
            }

            // This mpsc channel is used to collect the child process handles
            // that are generated by asynchronously spawned tasks in the
            // upcomming <task> loop.
            let (tx, rx) = channel::bounded::<Option<(u32, Child)>>(flow.tasks.len());

            // TODO: Handle this situation beforehand with some validation.
            // There is no point to allow the application to get executed this
            // far.
            if !flow.tasks.is_empty() {
                for task in &flow.tasks {
                    smol::spawn(run_task(
                        // Care **clone** calls.
                        tx.clone(),
                        task.clone(),
                        state.handles.as_ref().unwrap().clone(),
                        state.blueprints.as_ref().unwrap().clone(),
                        state.env.as_ref().unwrap().clone(),
                    ))
                    .detach();
                }

                // This for loop responsible for collecting the child process
                // handles that are sent by the mpsc channel declared before.
                // The collected handles are stored in the Application State,
                // so that they can be accessible by the other spawned tasks
                // and manage their ordering. (in terms of execution order)
                //
                // MENTAL NOTE: This code assumes that each Task sends exactly
                // one message through the channel. If a Task fails to send
                // its message, OR there happens to be a Task implemented to
                // not sent a message (wrong implementation) this loop would
                // hang which causes the entire application to hang.
                for _ in 0..flow.tasks.len() {
                    match rx.recv().await {
                        Ok(p) => {
                            if let Some((task_id, handle)) = p {
                                let mut handles = state.handles.as_ref().unwrap().lock().await;
                                handles.insert(task_id, handle);
                            }
                        }
                        Err(_) => return Err(anyhow!("CHANNEL ERROR")),
                    }
                }

                // After each Task gets spawned and their handles are stored,
                // the application makes sure that each of them get exited.
                //
                // TODO: If for some reason a spawned Task not gets exited with
                // success status, the application panics which is not
                // convenient. The application should shutdown gracefully.
                // Maybe even a Rollback can be implemented.
                let mut handles = state.handles.as_ref().unwrap().lock().await;
                for p in handles.iter_mut() {
                    match p.1.try_status() {
                        Ok(Some(status)) => info!("Task {} exited with: {status}", p.0),
                        Ok(None) => match p.1.status().await {
                            Ok(status) => info!("Task {} exited with: {status}", p.0),
                            Err(e) => panic!("Task {} exited with error: {e}", p.0),
                        },
                        Err(e) => panic!("error attempting to wait: {e}"),
                    }
                }
            } else {
                return Err(anyhow!("Flow should have at least one task."));
            }
        } else {
            return Err(anyhow!("No flow found for given idx."));
        }
    } else {
        // TODO: Handle this situation beforehand with some validation.
        // There is no point to allow the application to get executed this
        // far.
        return Err(anyhow!("Application state has no flow to run."));
    }
    Ok(())
}

/// Package Dependency Check:
/// Checks whether the execution environment has the necessary packages given
/// installed.
///
/// Returns error if
/// - It fails its internal <send> or <recv> calls (SendError, RecvError)
/// - It finds a missing dependency
pub async fn check_package_dependencies(deps: &Vec<String>) -> Result<()> {
    info!("Checking package dependencies");
    let (tx, rx) = channel::unbounded::<(String, Child)>();
    for d in deps {
        tx.send((
            d.clone(),
            Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {}", d.clone()))
                .stdout(Stdio::null())
                .spawn()?,
        ))
        .await?;
    }
    for _ in deps {
        match rx.recv().await {
            Ok((d, mut child)) => {
                if !child.status().await?.success() {
                    return Err(anyhow!("Missing dependency: {}", d));
                }
            }
            Err(_) => return Err(anyhow!("CHANNEL ERROR")),
        }
    }
    Ok(())
}
