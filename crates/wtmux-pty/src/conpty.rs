use anyhow::Result;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, INVALID_HANDLE_VALUE, S_OK,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
};
use windows_sys::Win32::System::Pipes::CreateNamedPipeW;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
    UpdateProcThreadAttribute, EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROCESS_INFORMATION, STARTUPINFOEXW,
};

const PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE: usize = 0x00020016;
const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const PIPE_ACCESS_INBOUND: u32 = 0x00000001;
const PIPE_ACCESS_OUTBOUND: u32 = 0x00000002;
static PIPE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A Windows ConPTY (pseudo-console) wrapper.
pub struct ConPty {
    hpc: HPCON,
    process_handle: OwnedHandle,
    thread_handle: OwnedHandle,
    input: tokio::net::windows::named_pipe::NamedPipeClient,
    output: tokio::net::windows::named_pipe::NamedPipeClient,
}

// Safety: ConPty handles are thread-safe when properly synchronized.
unsafe impl Send for ConPty {}
unsafe impl Sync for ConPty {}

impl ConPty {
    /// Create a pipe pair where our end supports FILE_FLAG_OVERLAPPED for async I/O.
    ///
    /// Anonymous pipes (CreatePipe) don't support overlapped I/O, which tokio
    /// requires for IOCP-based async reads/writes. We use named pipes instead.
    ///
    /// If `input` is true (ConPTY reads, we write async):
    ///   Returns (read_handle_for_conpty, overlapped_write_handle_for_us)
    ///
    /// If `input` is false (ConPTY writes, we read async):
    ///   Returns (write_handle_for_conpty, overlapped_read_handle_for_us)
    unsafe fn create_overlapped_pipe(input: bool) -> Result<(HANDLE, HANDLE)> {
        let id = PIPE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = if input { "in" } else { "out" };
        let name: Vec<u16> = format!("\\\\.\\pipe\\wtmux-pty-{}-{}-{}", dir, pid, id)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let access = if input {
            PIPE_ACCESS_INBOUND // server reads (ConPTY side)
        } else {
            PIPE_ACCESS_OUTBOUND // server writes (ConPTY side)
        };

        let server = CreateNamedPipeW(
            name.as_ptr(),
            access,
            0, // PIPE_TYPE_BYTE | PIPE_WAIT
            1,
            4096,
            4096,
            0,
            ptr::null(),
        );
        if server == INVALID_HANDLE_VALUE {
            anyhow::bail!(
                "CreateNamedPipeW failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let client_access = if input { GENERIC_WRITE } else { GENERIC_READ };
        let client = CreateFileW(
            name.as_ptr(),
            client_access,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED,
            ptr::null_mut(),
        );
        if client == INVALID_HANDLE_VALUE {
            CloseHandle(server);
            anyhow::bail!(
                "CreateFileW (pipe client) failed: {}",
                std::io::Error::last_os_error()
            );
        }

        Ok((server, client))
    }

    /// Spawn a new process in a ConPTY pseudo-console.
    pub fn spawn(command: &str, cols: u16, rows: u16) -> Result<Self> {
        unsafe {
            // Create overlapped pipe pairs for ConPTY I/O.
            let (pty_input_read, pty_input_write) = Self::create_overlapped_pipe(true)?;
            let (pty_output_write, pty_output_read) = Self::create_overlapped_pipe(false)?;

            // Create the pseudo-console.
            let size = COORD {
                X: cols as i16,
                Y: rows as i16,
            };
            let mut hpc: HPCON = 0;
            let hr = CreatePseudoConsole(size, pty_input_read, pty_output_write, 0, &mut hpc);
            if hr != S_OK {
                CloseHandle(pty_input_read);
                CloseHandle(pty_input_write);
                CloseHandle(pty_output_read);
                CloseHandle(pty_output_write);
                anyhow::bail!("CreatePseudoConsole failed: HRESULT 0x{:08x}", hr);
            }

            // Close the sides of the pipes that the ConPTY owns.
            CloseHandle(pty_input_read);
            CloseHandle(pty_output_write);

            // Initialize the startup info with the pseudo-console attribute.
            let mut attr_list_size: usize = 0;
            InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut attr_list_size);

            let mut attr_list_buf = vec![0u8; attr_list_size];
            let attr_list = attr_list_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;

            if InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_list_size) == 0 {
                ClosePseudoConsole(hpc);
                CloseHandle(pty_input_write);
                CloseHandle(pty_output_read);
                anyhow::bail!(
                    "InitializeProcThreadAttributeList failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            if UpdateProcThreadAttribute(
                attr_list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
                hpc as *const std::ffi::c_void,
                std::mem::size_of::<HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            ) == 0
            {
                DeleteProcThreadAttributeList(attr_list);
                ClosePseudoConsole(hpc);
                CloseHandle(pty_input_write);
                CloseHandle(pty_output_read);
                anyhow::bail!(
                    "UpdateProcThreadAttribute failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            let mut si: STARTUPINFOEXW = std::mem::zeroed();
            si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
            si.lpAttributeList = attr_list;

            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

            // Convert command to wide string.
            let cmd_wide: Vec<u16> = command
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut cmd_wide = cmd_wide;

            let success = CreateProcessW(
                ptr::null(),
                cmd_wide.as_mut_ptr(),
                ptr::null(),
                ptr::null(),
                0, // bInheritHandles = FALSE
                EXTENDED_STARTUPINFO_PRESENT,
                ptr::null(),
                ptr::null(),
                &si.StartupInfo,
                &mut pi,
            );

            DeleteProcThreadAttributeList(attr_list);

            if success == 0 {
                ClosePseudoConsole(hpc);
                CloseHandle(pty_input_write);
                CloseHandle(pty_output_read);
                anyhow::bail!(
                    "CreateProcessW failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            let process_handle = OwnedHandle::from_raw_handle(pi.hProcess as _);
            let thread_handle = OwnedHandle::from_raw_handle(pi.hThread as _);

            debug!(
                "ConPTY spawned: pid={}, cmd='{}', size={}x{}",
                pi.dwProcessId, command, cols, rows
            );

            // Wrap the overlapped pipe handles in tokio async types.
            let input = tokio::net::windows::named_pipe::NamedPipeClient::from_raw_handle(
                pty_input_write as _,
            )?;
            let output = tokio::net::windows::named_pipe::NamedPipeClient::from_raw_handle(
                pty_output_read as _,
            )?;

            Ok(ConPty {
                hpc,
                process_handle,
                thread_handle,
                input,
                output,
            })
        }
    }

    /// Resize the pseudo-console.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };
        let hr = unsafe { ResizePseudoConsole(self.hpc, size) };
        if hr != S_OK {
            anyhow::bail!("ResizePseudoConsole failed: HRESULT 0x{:08x}", hr);
        }
        debug!("ConPTY resized to {}x{}", cols, rows);
        Ok(())
    }

    /// Write data to the ConPTY input (keyboard input to the process).
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.input.write_all(data).await?;
        self.input.flush().await?;
        Ok(())
    }

    /// Read data from the ConPTY output (process output).
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.output.read(buf).await?;
        Ok(n)
    }

    /// Get the process handle.
    pub fn process_handle(&self) -> HANDLE {
        use std::os::windows::io::AsRawHandle;
        self.process_handle.as_raw_handle() as HANDLE
    }

    /// Wait for the process to exit.
    pub async fn wait(&self) -> Result<u32> {
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, WaitForSingleObject, INFINITE,
        };

        let handle = self.process_handle() as isize;
        // Spawn a blocking task since WaitForSingleObject blocks.
        let exit_code = tokio::task::spawn_blocking(move || unsafe {
            let h = handle as HANDLE;
            WaitForSingleObject(h, INFINITE);
            let mut exit_code: u32 = 0;
            GetExitCodeProcess(h, &mut exit_code);
            exit_code
        })
        .await?;

        Ok(exit_code)
    }
}

impl Drop for ConPty {
    fn drop(&mut self) {
        debug!("Closing ConPTY");
        unsafe {
            ClosePseudoConsole(self.hpc);
        }
    }
}
