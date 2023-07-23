use std::{
    ffi::OsString,
};
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};

const SERVICE_NAME: &str = "tusk-server";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
pub const CONFIGURATION_FILE_PATH: &str = "C:\\ProgramData\\Tusk\\tusk.toml";

define_windows_service!(ffi_service_main, tusk_server_main);

pub fn run() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

pub fn initialize_logger() {
    winlog::init("Tusk Server")
        .expect("a functioning logger");
}

pub fn tusk_server_main(_arguments: Vec<OsString>) {
    if let Err(_) = run_service() {
        // TODO
    }
}

pub fn run_service() -> Result<()> {
    let server = crate::server_spawn()
        .expect("server");
    let handle = server.handle();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                actix_web::rt::System::new().block_on(handle.stop(true));
                ServiceControlHandlerResult::NoError
            },
            _ => ServiceControlHandlerResult::NotImplemented
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None
    })?;

    crate::server_run(server)
        .expect("running server instance");

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