//! Defines the necessary functions to make the server run as a Windows Service.

use std::{
    ffi::OsString,
};
use std::time::Duration;
use log::LevelFilter;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

use tusk_core::error::TuskResult;

const SERVICE_NAME: &str = "tusk-server";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, tusk_server_main);

/// Runs the server.
pub fn run() -> TuskResult<()> {
    let mut run_cli = false;
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--cli" { run_cli = true; }
    }
    if run_cli {
        env_logger::builder()
            .filter_level(LevelFilter::Info)
            .init();

        let (server, tusk) = crate::server_spawn()?;

        let _w = super::spawn_watcher(tusk);

        crate::server_run(server)?;
    } else {
        winlog::init("Tusk Server").unwrap();

        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    }

    Ok(())
}
/// Wraps the function [`run_service`] so that any error occurred during the initialization phase
/// is logged.
pub fn tusk_server_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        log::error!("{e}");
    }
}

/// Runs the main server as a Windows Service.
pub fn run_service() -> TuskResult<()> {
    let (server, tusk) = crate::server_spawn()?;
    let handle = server.handle();

    super::spawn_watcher(tusk.clone());

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                actix_web::rt::System::new().block_on(handle.stop(true));
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::Pause => {
                actix_web::rt::System::new().block_on(handle.pause());
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::Continue => {
                let mut tera = match tusk.tera_mut() {
                    Ok(lock) => lock,
                    Err(e) => {
                        log::error!("{e}");
                        return ServiceControlHandlerResult::Other(1);
                    }
                };
                match tera.full_reload() {
                    Ok(()) => {},
                    Err(e) => {
                        log::error!("{e}");
                        return ServiceControlHandlerResult::Other(2);
                    }
                };
                actix_web::rt::System::new().block_on(handle.resume());
                ServiceControlHandlerResult::NoError
            },
            _ => ServiceControlHandlerResult::NotImplemented
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::PAUSE_CONTINUE,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None
    })?;

    crate::server_run(server)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None
    })?;

    Ok(())
}