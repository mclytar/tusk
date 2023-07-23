use std::ffi::OsString;
use std::time::{Duration, Instant};
use windows_service::{
    Error as ServiceError,
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess}
};
use windows_service::service::ServiceState;

pub fn daemon_install() -> windows_service::Result<()> {

    println!("Gathering information...");

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("tusk-server.exe");

    let service_info = ServiceInfo {
        name: OsString::from("tusk-server"),
        display_name: OsString::from("Tusk server [development]"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::OnDemand,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None,
        account_password: None
    };

    println!("Installing server...");

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("Tusk server service for development")?;

    println!("Done!");

    Ok(())
}

pub fn daemon_uninstall() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.delete()?;

    if service.query_status()?.current_state != ServiceState::Stopped {
        service.stop()?;
    }

    drop(service);

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while start.elapsed() < timeout {
        print!(".");
        if let Err(windows_service::Error::Winapi(e)) = service_manager.open_service("tusk-server", ServiceAccess::QUERY_STATUS) {
            if e.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("\nService uninstalled successfully!");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    println!("\nCannot verify deletion status.");
    println!("Service tusk-server has been marked for deletion.");

    Ok(())
}

pub fn daemon_start() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.start::<OsString>(&[])?;

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while start.elapsed() < timeout {
        print!(".");
        match service.query_status()?.current_state {
            ServiceState::Running => {
                println!("\nService started successfully!");
                return Ok(());
            },
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    println!("\nCannot verify success of the operation.");

    Ok(())
}

pub fn daemon_stop() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.stop()?;

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while start.elapsed() < timeout {
        print!(".");
        match service.query_status()?.current_state {
            ServiceState::Stopped => {
                println!("\nService stopped successfully!");
                return Ok(());
            },
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    println!("\nCannot verify success of the operation.");

    Ok(())
}

pub fn daemon_reload() -> windows_service::Result<()> {
    daemon_stop()?;
    daemon_start()?;
    Ok(())
}

pub fn print_error(e: ServiceError) {
    match e {
        ServiceError::Winapi(e) => println!("Cannot perform operation: {e}"),
        _ => println!("Cannot perform operation: {e}")
    }
}