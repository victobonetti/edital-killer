use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::Notify;

/// Pedido de cancelamento compartilhado entre quem cancela (UI, fechar o app) e o job.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<(AtomicBool, Notify)>);

impl CancelToken {
    pub fn cancelar(&self) {
        self.0 .0.store(true, Ordering::SeqCst);
        self.0 .1.notify_waiters();
    }

    pub fn cancelado(&self) -> bool {
        self.0 .0.load(Ordering::SeqCst)
    }

    /// Resolve quando `cancelar` for chamado (na hora, se já foi).
    pub async fn esperar(&self) {
        loop {
            let notificado = self.0 .1.notified();
            if self.cancelado() {
                return;
            }
            notificado.await;
        }
    }
}
