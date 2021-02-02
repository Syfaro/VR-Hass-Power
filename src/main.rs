//! A program to monitor the state of a process and control a Home Assistant
//! entity based on the state.

use std::sync::{mpsc, Arc, Mutex};
use sysinfo::{ProcessExt, SystemExt};

mod config;
mod hass;

use config::CheckConfig;
use hass::{set_entity_state, APIState};

/// A thread-safe mutable [`sysinfo::System`].
type SystemInfo = Arc<Mutex<sysinfo::System>>;

/// The current state of the VR process. When running, it includes the pid.
#[derive(Clone, Copy, Debug, PartialEq)]
enum VRState {
    /// The process is not running.
    NotRunning,
    /// The process is running and has the specified pid.
    Running(usize),
}

/// Get the state of the VR process, refreshing data and lookup up the process
/// by name.
fn get_initial_state(config: &CheckConfig, system_info: SystemInfo) -> VRState {
    let mut system = system_info.lock().unwrap();
    system.refresh_processes();

    match system.get_process_by_name(&config.process_name).first() {
        Some(process) => VRState::Running(process.pid()),
        None => VRState::NotRunning,
    }
}

/// Check the state of the VR process, refreshing data as frequently as
/// specified in the configuration. Events are only sent on changes. It returns
/// a tuple containing the current state and if the value is the initial value.
fn poll_vr_state_updates(
    config: CheckConfig,
    system_info: SystemInfo,
) -> mpsc::Receiver<(VRState, bool)> {
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        // Get initial state to initialize values and inform program what the
        // value was before starting the program.
        let mut state = get_initial_state(&config, system_info.clone());
        tracing::info!(?state, "Got initial state");
        tx.send((state, true)).unwrap();

        // Loop forever, checking the state of the VR process.
        loop {
            let old_state = state;

            let mut system = system_info.lock().unwrap();
            system.refresh_processes();

            // If the process was not previously running, we need to look up the
            // process by name because we do not know the pid. If we have the
            // pid we can lookup the process by that instead.
            match state {
                VRState::NotRunning => {
                    if let Some(process) = system.get_process_by_name(&config.process_name).first()
                    {
                        state = VRState::Running(process.pid())
                    }
                }

                VRState::Running(pid) => {
                    if system.get_process(pid).is_none() {
                        state = VRState::NotRunning
                    }
                }
            }

            drop(system);

            tracing::trace!(?state, ?old_state, "Updated state");

            // Only send updates on changes.
            if state != old_state {
                tracing::debug!(?state, "Got new state");
                tx.send((state, false)).unwrap();
            }

            std::thread::sleep(std::time::Duration::from_secs(config.interval));
        }
    });

    rx
}

fn main() {
    // Ensure useful information is displayed.
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "warn,vr_hass_power=info");
    }

    tracing_subscriber::fmt::init();

    // Create a directory to store the configuration file.
    let project_dir = directories::ProjectDirs::from("net.syfaro", "Syfaro", "VR Hass Power")
        .expect("Project directory could not be constructed");
    let config_dir = project_dir.config_dir();
    std::fs::create_dir_all(config_dir).expect("Project directory was unable to be created");

    // Try loading the configuration file, otherwise prompt the user for
    // essential information.
    let config = match config::load_config(&config_dir) {
        Ok(config) => {
            tracing::trace!("Loaded configuration");
            config
        }
        Err(err) => {
            tracing::debug!("Config load error: {:?}", err);
            tracing::warn!("Configuration was unable to be loaded, running setup");
            config::prompt_config(&config_dir).expect("Configuration was unable to be saved")
        }
    };

    let system_info = Arc::new(Mutex::new(sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_processes(),
    )));

    let updates = poll_vr_state_updates(config.check.clone(), system_info);

    loop {
        // Wait for the next state, blocking until a value is available. There
        // will always be an initial value available to ensure the current
        // device state is correct.
        let state = updates.recv().unwrap();
        tracing::info!(?state, "VR state update");

        match state {
            // If the state has changed to running, turn on the entity. It does
            // not matter if this was an initial value or not.
            (VRState::Running(_pid), _) => set_entity_state(&config.homeassistant, APIState::On)
                .expect("Unable to turn entity on"),
            // If VR is not running and this is the initial state, ensure the
            // devices are off.
            (VRState::NotRunning, true) => set_entity_state(&config.homeassistant, APIState::Off)
                .expect("Unable to turn entity off"),
            // If VR is not running and this is not the initial value, wait for
            // up to some number seconds for a new state to come in before
            // turning the devices off.
            (VRState::NotRunning, false) => {
                tracing::debug!(
                    delay = config.power.delay,
                    "Waiting to ensure software is not being restarted"
                );

                // If we get a new value that is still not running (this should
                // not be possible) or we have a timeout, turn off devices.
                // Otherwise, the new value suggests things are running again
                // and devices should not be turned off.
                match updates.recv_timeout(std::time::Duration::from_secs(config.power.delay)) {
                    Ok((VRState::NotRunning, _)) | Err(_) => {
                        tracing::info!("Turning off devices");
                        set_entity_state(&config.homeassistant, APIState::Off)
                            .expect("Unable to turn entity off");
                    }
                    _ => tracing::info!("Did not need to turn off devices"),
                }
            }
        }
    }
}
