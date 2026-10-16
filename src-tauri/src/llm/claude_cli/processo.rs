//! Garante que nenhum processo `claude` (nem filho dele) sobreviva a um cancelamento
//! ou ao fechamento do app.
//!
//! - Windows: um Job Object com `KILL_ON_JOB_CLOSE` por execução. `matar` encerra o
//!   Job Object inteiro; se o app morrer, o Windows fecha o handle e mata a árvore.
//! - Unix: o filho roda num grupo de processos próprio; `matar` manda `SIGKILL` ao grupo.

use tokio::process::{Child, Command};

/// Prepara o comando antes do `spawn`.
pub fn preparar(cmd: &mut Command) {
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        cmd.process_group(0);
    }
}

pub struct Arvore {
    #[cfg(windows)]
    job: win::Job,
    #[cfg(unix)]
    pgid: Option<i32>,
}

impl Arvore {
    /// Anexa o processo recém-criado. Falhar aqui não impede a execução: só perde a
    /// garantia de matar os netos, e isso fica no log.
    pub fn anexar(child: &Child) -> Self {
        #[cfg(windows)]
        {
            let job = win::Job::novo();
            if let Some(h) = child.raw_handle() {
                if let Err(e) = job.atribuir(h) {
                    tracing::warn!(erro = %e, "não consegui anexar o claude ao Job Object");
                }
            }
            Self { job }
        }
        #[cfg(unix)]
        {
            Self {
                pgid: child.id().and_then(|p| i32::try_from(p).ok()),
            }
        }
    }

    pub fn matar(&self) {
        #[cfg(windows)]
        self.job.terminar();
        #[cfg(unix)]
        if let Some(pgid) = self.pgid {
            // SAFETY: killpg só envia um sinal; pgid é o grupo criado para este filho.
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
        }
    }
}

#[cfg(windows)]
mod win {
    use std::os::windows::io::RawHandle;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    pub struct Job(HANDLE);

    // SAFETY: o handle de um Job Object pode ser usado de qualquer thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn novo() -> Self {
            // SAFETY: chamadas Win32 com ponteiros nulos ou para struct local inicializada.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if !h.is_null() {
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    SetInformationJobObject(
                        h,
                        JobObjectExtendedLimitInformation,
                        std::ptr::from_ref(&info).cast(),
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                }
                Self(h)
            }
        }

        pub fn atribuir(&self, processo: RawHandle) -> std::io::Result<()> {
            if self.0.is_null() {
                return Err(std::io::Error::other("CreateJobObjectW falhou"));
            }
            // SAFETY: ambos os handles são válidos enquanto o filho existe.
            if unsafe { AssignProcessToJobObject(self.0, processo as HANDLE) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }

        pub fn terminar(&self) {
            if !self.0.is_null() {
                // SAFETY: handle válido até o Drop.
                unsafe { TerminateJobObject(self.0, 1) };
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: fechar o último handle mata o que restar (KILL_ON_JOB_CLOSE).
                unsafe { CloseHandle(self.0) };
            }
        }
    }
}
