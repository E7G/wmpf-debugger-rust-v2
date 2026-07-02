use std::sync::Once;

#[allow(non_camel_case_types, dead_code)]
mod ffi_types {
    pub type gpointer = *mut std::ffi::c_void;
    pub type gboolean = std::ffi::c_int;
    pub type gint = std::ffi::c_int;
    pub type guint = std::ffi::c_uint;
    pub type gchar = std::ffi::c_char;
}

use ffi_types::*;

pub enum FridaDeviceManager {}
pub enum FridaDeviceList {}
pub enum FridaDevice {}
pub enum FridaProcessList {}
pub enum FridaProcess {}
pub enum FridaSession {}
pub enum FridaSessionOptions {}
pub enum FridaScript {}
pub enum FridaScriptOptions {}
pub enum FridaProcessQueryOptions {}
pub enum GError {}
pub enum GCancellable {}
pub enum GHashTable {}

#[repr(C)]
pub struct GErrorInternal {
    pub domain: u32,
    pub code: i32,
    pub message: *const gchar,
}

static INIT: Once = Once::new();

#[allow(non_camel_case_types, dead_code)]
extern "C" {
    pub fn frida_init();
    pub fn frida_deinit();

    pub fn frida_device_manager_new() -> *mut FridaDeviceManager;
    pub fn frida_device_manager_enumerate_devices_sync(
        self_: *mut FridaDeviceManager,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut FridaDeviceList;

    pub fn frida_device_list_size(self_: *mut FridaDeviceList) -> gint;
    pub fn frida_device_list_get(self_: *mut FridaDeviceList, index: gint) -> *mut FridaDevice;

    pub fn frida_device_get_dtype(self_: *mut FridaDevice) -> guint;

    pub fn frida_device_enumerate_processes_sync(
        self_: *mut FridaDevice,
        options: *mut FridaProcessQueryOptions,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut FridaProcessList;

    pub fn frida_device_attach_sync(
        self_: *mut FridaDevice,
        pid: guint,
        options: *mut FridaSessionOptions,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut FridaSession;

    pub fn frida_process_list_size(self_: *mut FridaProcessList) -> gint;
    pub fn frida_process_list_get(self_: *mut FridaProcessList, index: gint) -> *mut FridaProcess;

    pub fn frida_process_get_pid(self_: *mut FridaProcess) -> guint;
    pub fn frida_process_get_name(self_: *mut FridaProcess) -> *const gchar;
    pub fn frida_process_get_parameters(self_: *mut FridaProcess) -> *mut GHashTable;

    pub fn frida_session_create_script_sync(
        self_: *mut FridaSession,
        source: *const gchar,
        options: *mut FridaScriptOptions,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> *mut FridaScript;

    pub fn frida_script_load_sync(
        self_: *mut FridaScript,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    );
    pub fn frida_script_is_destroyed(self_: *mut FridaScript) -> gboolean;

    pub fn frida_session_options_new() -> *mut FridaSessionOptions;
    pub fn frida_process_query_options_new() -> *mut FridaProcessQueryOptions;
    pub fn frida_process_query_options_set_scope(
        self_: *mut FridaProcessQueryOptions,
        value: guint,
    );

    pub fn g_object_unref(object: gpointer);
    pub fn g_error_free(error: *mut GError);
    pub fn g_hash_table_lookup(hash_table: *mut GHashTable, key: gpointer) -> gpointer;
    pub fn g_variant_get_string(value: gpointer, length: *mut usize) -> *const gchar;
    pub fn g_variant_get_uint32(value: gpointer) -> u32;
}

pub fn init() {
    INIT.call_once(|| unsafe {
        frida_init();
    });
}

fn extract_error(error: *mut GError) -> String {
    unsafe {
        if error.is_null() {
            return "unknown error".to_string();
        }
        let err = &*(error as *const GErrorInternal);
        let msg = if err.message.is_null() {
            "unknown error".to_string()
        } else {
            std::ffi::CStr::from_ptr(err.message)
                .to_string_lossy()
                .to_string()
        };
        g_error_free(error);
        msg
    }
}

pub struct DeviceManager {
    inner: *mut FridaDeviceManager,
}

unsafe impl Send for DeviceManager {}
unsafe impl Sync for DeviceManager {}

impl Drop for DeviceManager {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.inner as gpointer);
        }
    }
}

impl DeviceManager {
    pub fn new() -> Result<Self, String> {
        init();
        let inner = unsafe { frida_device_manager_new() };
        if inner.is_null() {
            return Err("failed to create device manager".into());
        }
        Ok(Self { inner })
    }

    pub fn get_local_device(&self) -> Result<Device, String> {
        let mut error: *mut GError = std::ptr::null_mut();
        let device_list = unsafe {
            frida_device_manager_enumerate_devices_sync(
                self.inner,
                std::ptr::null_mut(),
                &mut error,
            )
        };

        if !error.is_null() {
            return Err(extract_error(error));
        }
        if device_list.is_null() {
            return Err("failed to enumerate devices".into());
        }

        let size = unsafe { frida_device_list_size(device_list) };
        for i in 0..size {
            let device = unsafe { frida_device_list_get(device_list, i) };
            if device.is_null() {
                continue;
            }

            // FRIDA_DEVICE_TYPE_LOCAL = 0
            if unsafe { frida_device_get_dtype(device) } == 0 {
                let result = Device { inner: device };
                unsafe {
                    g_object_unref(device_list as gpointer);
                }
                return Ok(result);
            }
        }

        unsafe {
            g_object_unref(device_list as gpointer);
        }
        Err("local device not found".into())
    }
}

pub struct Device {
    inner: *mut FridaDevice,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.inner as gpointer);
        }
    }
}

impl Device {
    pub fn enumerate_processes(&self) -> Result<Vec<Process>, String> {
        let options = unsafe { frida_process_query_options_new() };
        // FRIDA_SCOPE_METADATA = 1
        unsafe {
            frida_process_query_options_set_scope(options, 1);
        }

        let mut error: *mut GError = std::ptr::null_mut();
        let process_list = unsafe {
            frida_device_enumerate_processes_sync(
                self.inner,
                options,
                std::ptr::null_mut(),
                &mut error,
            )
        };
        unsafe {
            g_object_unref(options as gpointer);
        }

        if !error.is_null() {
            return Err(extract_error(error));
        }
        if process_list.is_null() {
            return Err("failed to enumerate processes".into());
        }

        let size = unsafe { frida_process_list_size(process_list) };
        let mut processes = Vec::new();

        for i in 0..size {
            let process = unsafe { frida_process_list_get(process_list, i) };
            if process.is_null() {
                continue;
            }

            let pid = unsafe { frida_process_get_pid(process) };
            let name = unsafe {
                let ptr = frida_process_get_name(process);
                if ptr.is_null() {
                    String::new()
                } else {
                    std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string()
                }
            };

            let params = unsafe { frida_process_get_parameters(process) };
            let mut path = String::new();
            let mut ppid: guint = 0;

            if !params.is_null() {
                unsafe {
                    let key_path = std::ffi::CString::new("path").unwrap();
                    let val = g_hash_table_lookup(params, key_path.as_ptr() as gpointer);
                    if !val.is_null() {
                        let ptr = g_variant_get_string(val as gpointer, std::ptr::null_mut());
                        if !ptr.is_null() {
                            path = std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string();
                        }
                    }

                    let key_ppid = std::ffi::CString::new("ppid").unwrap();
                    let val = g_hash_table_lookup(params, key_ppid.as_ptr() as gpointer);
                    if !val.is_null() {
                        ppid = g_variant_get_uint32(val as gpointer);
                    }
                }
            }

            processes.push(Process {
                pid,
                name,
                path,
                ppid,
            });
        }

        unsafe {
            g_object_unref(process_list as gpointer);
        }
        Ok(processes)
    }

    pub fn attach(&self, pid: guint) -> Result<Session, String> {
        let options = unsafe { frida_session_options_new() };
        let mut error: *mut GError = std::ptr::null_mut();

        let session = unsafe {
            frida_device_attach_sync(self.inner, pid, options, std::ptr::null_mut(), &mut error)
        };
        unsafe {
            g_object_unref(options as gpointer);
        }

        if !error.is_null() {
            return Err(extract_error(error));
        }
        if session.is_null() {
            return Err("failed to attach".into());
        }

        Ok(Session { inner: session })
    }
}

pub struct Process {
    pub pid: guint,
    pub name: String,
    pub path: String,
    pub ppid: guint,
}

pub struct Session {
    inner: *mut FridaSession,
}

unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.inner as gpointer);
        }
    }
}

impl Session {
    pub fn create_script(&self, source: &str) -> Result<Script, String> {
        let c_source = std::ffi::CString::new(source).map_err(|e| e.to_string())?;
        let mut error: *mut GError = std::ptr::null_mut();

        let script = unsafe {
            frida_session_create_script_sync(
                self.inner,
                c_source.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut error,
            )
        };

        if !error.is_null() {
            return Err(extract_error(error));
        }
        if script.is_null() {
            return Err("failed to create script".into());
        }

        Ok(Script { inner: script })
    }
}

pub struct Script {
    inner: *mut FridaScript,
}

unsafe impl Send for Script {}
unsafe impl Sync for Script {}

impl Drop for Script {
    fn drop(&mut self) {
        unsafe {
            if frida_script_is_destroyed(self.inner) == 0 {
                g_object_unref(self.inner as gpointer);
            }
        }
    }
}

impl Script {
    pub fn load(&self) -> Result<(), String> {
        let mut error: *mut GError = std::ptr::null_mut();
        unsafe {
            frida_script_load_sync(self.inner, std::ptr::null_mut(), &mut error);
        }
        if !error.is_null() {
            return Err(extract_error(error));
        }
        Ok(())
    }
}
