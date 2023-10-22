use config::{Config, File, FileFormat};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct Conf {
    blueprints: Option<HashMap<String, Blueprint>>,
    env: Option<HashMap<String, Vec<(String, String)>>>,
    flows: Option<Vec<Flow>>,
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
    hc: Option<HealthCheck>,
}

#[derive(Deserialize)]
struct HealthCheck {
    cmd: (ExecutionEnvironment, String),
    interval: Option<String>,
    retries: Option<u32>,
}

#[derive(Deserialize)]
struct Blueprint {
    _env: Option<Vec<(String, String)>>,
    image: Option<Image>,
    container: Option<Container>,
}

#[derive(Deserialize, Clone)]
struct Flow {
    name: String,
    tasks: Vec<Task>,
    pkg_dependencies: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
struct Task {
    id: u32,
    #[serde(rename = "type")]
    typ: TaskType,
    name: String,
    job: JobType,
    depends: Option<u32>,
}

#[derive(Deserialize, Clone)]
enum TaskType {
    Blueprint,
    Env,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
enum JobType {
    Container,
    Image,
    Set,
}

fn main() {
    let start = Instant::now();
    // TODO: Change this
    let conf: Conf = Config::builder()
        .add_source(File::new(".runer", FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize::<Conf>()
        .unwrap();
    let local_global_env = conf.env.unwrap_or_else(|| HashMap::new());
    if let Some(blueprints) = conf.blueprints {
        if let Some(flows) = conf.flows {
            run_flows(blueprints, flows, local_global_env);
        }
    }
    // ========================================================================
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
}

fn run_flows(
    blueprints: HashMap<String, Blueprint>,
    flows: Vec<Flow>,
    local_global_env: HashMap<String, Vec<(String, String)>>,
) {
    let f = flows[0].clone();
    // flows.iter().for_each(|f| {
    if let Some(pkg_dependencies) = &f.pkg_dependencies {
        check_package_dependencies(pkg_dependencies);
    }

    let handles = Arc::new(Mutex::new(HashMap::<u32, Child>::new()));
    let blueprints = Arc::new(blueprints);
    let local_global_env = Arc::new(local_global_env);
    let mut thread_handlers: Vec<JoinHandle<()>> = vec![];

    if !f.tasks.is_empty() {
        for t in f.tasks {
            let handles = handles.clone();
            let blueprints = blueprints.clone();
            let local_global_env = local_global_env.clone();
            if let Some(_) = t.depends {
                let thread_handler = thread::spawn(move || {
                    run_task(t, handles, blueprints, local_global_env);
                });
                thread_handlers.push(thread_handler);
            } else {
                run_task(t, handles, blueprints, local_global_env);
            }
        }

        for h in thread_handlers {
            h.join().unwrap();
        }

        // Making sure every command exited gracefully
        if let Ok(mut handles) = handles.lock() {
            handles.iter_mut().for_each(|p| {
                p.1.wait()
                    .unwrap_or_else(|e| panic!("There are unsuccessfull commands. Error: {}", e));
            });
        }
    } else {
        panic!("Flow has to have at least one task.");
    }
    // })
}

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
        thread::sleep(Duration::from_millis(50));
    }

    // Then it makes sure that the parent task finishes successfully ==========
    if let Ok(mut handles) = handles.lock() {
        let handle = handles.get_mut(&parent_handle_id).unwrap_or_else(|| {
            panic!(
                "Task {} can not find the parent process handle",
                child_task_id
            )
        });
        println!(
            "Waiting for task_id = {} to finish to start task_id = {} ",
            parent_handle_id, child_task_id
        );

        handle
            .wait()
            .expect("Something went wrong waiting for parent process");
        println!(
            "Waited for task_id = {} to finish to start task_id = {} ",
            parent_handle_id, child_task_id
        );
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

/// Creates a new docker image(if it doesn't exist) according to given Image.
///
/// ---
/// Panics if <docker build> command returns non-success code.
fn create_docker_image(docker_image: &Image) -> Child {
    println!("Starting to create docker image for {}", docker_image.tag);
    let mut commands: Vec<String> = vec![];
    if let Some(pre) = &docker_image.pre {
        let mut cmds = create_commands_from_tokens(pre);
        cmds.iter_mut().for_each(|cmd| {
            commands.push(format!("{:?}", cmd.current_dir(&docker_image.context)));
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

    commands.push(format!(
        "{:?}",
        docker_build_command.arg(&docker_image.context)
    ));

    if let Some(post) = &docker_image.post {
        let mut cmds = create_commands_from_tokens(&post);
        cmds.iter_mut().for_each(|cmd| {
            commands.push(format!("{:?}", cmd.current_dir(&docker_image.context)));
        });
    }
    Command::new("sh")
        .arg("-c")
        .arg(commands.join(" && "))
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
        if !hc.cmd.1.is_empty() {
            match &hc.cmd.0 {
                // TODO: This is wrong if it is Local should be handled differently
                // --health-cmd is a docker run command option
                ExecutionEnvironment::Local => {
                    docker_run_command.args(["--health-cmd", &hc.cmd.1]);
                }
                ExecutionEnvironment::Container(_container_identification) => {
                    todo!("Implement command execution in Container")
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
