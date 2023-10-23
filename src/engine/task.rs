use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use smol::channel::Sender;
use smol::lock::Mutex;
use smol::process::Child;
use smol::Timer;

use crate::model::runer::{Blueprint, JobType, Task, TaskType};

use super::job::{
    create_docker_image, run_docker_container, run_shell_script, set_environment_variables,
};

pub async fn run_task(
    tx: Sender<Option<(u32, Child)>>,
    task: Task,
    handles: Arc<Mutex<HashMap<u32, Child>>>,
    blueprints: Arc<HashMap<String, Blueprint>>,
    env: Arc<HashMap<String, Vec<(String, String)>>>,
) {
    // TODO: Implement waiting for parent process
    if let Some(depends) = task.depends {
        wait_until_parent_task_command_is_finished(depends, task.id, handles.clone()).await;
    }

    match task.typ {
        TaskType::Blueprint => {
            let (_, blueprint) = blueprints.get_key_value(&task.name).unwrap_or_else(|| {
                panic!(
                    "Task ID:{}, Name: {}, no bluprint found",
                    task.id, task.name
                );
            });
            let handle = match task.job {
                JobType::Image => {
                    create_docker_image(blueprint.image.as_ref().unwrap_or_else(|| {
                        panic!(
                            "Task ID: {}, Name: {}, no image job found",
                            task.id, task.name
                        );
                    }))
                    .await
                }
                JobType::Container => {
                    run_docker_container(blueprint.container.as_ref().unwrap_or_else(|| {
                        panic!(
                            "Task ID: {}, Name: {}, no container job found",
                            task.id, task.name
                        );
                    }))
                }
                JobType::Set => {
                    todo!("Decide how to handle 'Set' jobs inside blueprints");
                }
                JobType::Shell => {
                    run_shell_script(blueprint.shell.as_ref().unwrap_or_else(|| {
                        panic!(
                            "Task ID: {}, Name: {}, no shell job found",
                            task.id, task.name
                        );
                    }))
                    .await
                }
            };
            let _ = tx
                .send(Some((
                    task.id,
                    handle.unwrap_or_else(|_| {
                        panic!("FATAL ERROR: Something wrong with process handles")
                    }),
                )))
                .await;
        }
        TaskType::Env => {
            let (_, env) = env.get_key_value(&task.name).unwrap_or_else(|| {
                panic!(
                    "Task ID: {}, Name: {}, no environment variable list is found",
                    task.id, task.name
                );
            });
            let _ = tx
                .send(Some((
                    task.id,
                    set_environment_variables(env).unwrap_or_else(|_| {
                        panic!("FATAL ERROR: Something wrong with process handles")
                    }),
                )))
                .await;
        }
    }
}

async fn wait_until_parent_task_command_is_finished(
    parent_handle_id: u32,
    child_task_id: u32,
    handles: Arc<Mutex<HashMap<u32, Child>>>,
) {
    // Here it waits until the parent handle becomes available ================
    let mut available = false;
    while !available {
        let handles = handles.lock().await;
        if handles.contains_key(&parent_handle_id) {
            available = true;
        }
        Timer::after(Duration::from_millis(5)).await;
    }

    // Then it makes sure that the parent task finishes successfully ==========
    let mut finished_successfully = false;
    while !finished_successfully {
        let mut handles = handles.lock().await;
        let handle = handles.get_mut(&parent_handle_id).unwrap_or_else(|| {
            panic!(
                "Task {} can not find the parent process handle",
                child_task_id
            )
        });
        match handle.try_status() {
            Ok(Some(status)) => {
                if status.success() {
                    finished_successfully = true;
                }
            }
            Ok(None) => {}
            Err(e) => panic!("error attempting to wait: {e}"),
        }
        Timer::after(Duration::from_millis(5)).await;
    }
}
