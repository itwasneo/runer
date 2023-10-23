use anyhow::{anyhow, Result};
use log::info;
use smol::channel;
use smol::process::{Child, Command, Stdio};

use super::state::State;
use super::task::run_task;

pub async fn execute_flow(flow_idx: usize, state: State) -> Result<()> {
    if let Some(flows) = state.flows {
        if let Some(flow) = flows.get(flow_idx) {
            if let Some(pkg_dependencies) = &flow.pkg_dependencies {
                check_package_dependencies(&pkg_dependencies).await?;
            }

            let (tx, rx) = channel::bounded::<Option<(u32, Child)>>(flow.tasks.len());

            if !flow.tasks.is_empty() {
                for task in &flow.tasks {
                    let task = task.clone();
                    let handles = state.handles.as_ref().unwrap().clone();
                    let blueprints = state.blueprints.as_ref().unwrap().clone();
                    let env = state.env.as_ref().unwrap().clone();
                    let tx = tx.clone();
                    // run_task(tx, task, handles, blueprints, env).await;
                    smol::spawn(run_task(tx, task, handles, blueprints, env)).detach();
                }

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

                // Making sure every command exited gracefully
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
    let (tx, rx) = channel::unbounded::<Child>();
    for d in deps {
        let d = d.clone();
        tx.send(
            Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {}", d))
                .stdout(Stdio::null())
                .spawn()?,
        )
        .await?;
    }
    for _ in deps {
        match rx.recv().await {
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
