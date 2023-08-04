use std::ffi::OsString;
use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};
use windows_service::{
    Error as ServiceError,
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess}
};
use windows_service::service::ServiceState;

use tusk_backend::error::Result;

pub fn service_install() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Gathering information...");

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

    pb.println("  Information gathered.");
    pb.set_message("Installing server...");

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("Tusk server service for development")?;

    pb.println("  Server installed.");
    pb.set_message("Registering logger...");

    if let Err(e) = winlog::try_register("Tusk Server") {
        pb.println(format!("  Cannot register logger: {e}"));
    }

    pb.finish_with_message("Done!");

    Ok(())
}

pub fn service_uninstall() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Uninstalling...");

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
        if let Err(windows_service::Error::Winapi(e)) = service_manager.open_service("tusk-server", ServiceAccess::QUERY_STATUS) {
            if e.raw_os_error() == Some(windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                pb.finish_with_message("Service uninstalled successfully.");
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    pb.finish_with_message("Cannot verify deletion status.");
    println!("Service tusk-server has been marked for deletion.");

    println!("Deregister logger...");

    if let Err(e) = winlog::try_deregister("Tusk Server") {
        println!("Cannot deregister logger: {e}");
    };

    println!("Done!");

    Ok(())
}

pub fn service_start() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Starting server...");

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::START;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.start::<OsString>(&[])?;

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while start.elapsed() < timeout {
        match service.query_status()?.current_state {
            ServiceState::Running => {
                pb.finish_with_message("Service started successfully!");
                return Ok(());
            },
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    pb.finish_with_message("Cannot verify success of the operation.");

    Ok(())
}

pub fn service_stop() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Stopping server...");

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.stop()?;

    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while start.elapsed() < timeout {
        match service.query_status()?.current_state {
            ServiceState::Stopped => {
                pb.finish_with_message("Service stopped successfully!");
                return Ok(());
            },
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    pb.finish_with_message("Cannot verify success of the operation.");

    Ok(())
}

pub fn service_reload() -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap().tick_chars("|/-\\ "));
    pb.enable_steady_tick(Duration::from_millis(50));
    pb.set_message("Querying server...");

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::PAUSE_CONTINUE;
    let service = service_manager.open_service("tusk-server", service_access)?;

    service.pause()?;
    service.resume()?;

    pb.finish_with_message("Done!");

    Ok(())
}

pub fn print_error(e: tusk_backend::error::Error) {
    match e {
        tusk_backend::error::Error::WindowsServiceError(ServiceError::Winapi(e)) => println!("Cannot perform operation: {e}"),
        _ => println!("Cannot perform operation: {e}")
    }
}