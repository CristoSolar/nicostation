//! Deshacer / rehacer con instantáneas del proyecto serializado.

const MAX: usize = 200;

pub struct Historial {
    undo: Vec<String>,
    redo: Vec<String>,
    /// Último estado confirmado.
    cur: String,
}

impl Historial {
    pub fn new(cur: String) -> Self {
        Historial { undo: vec![], redo: vec![], cur }
    }

    /// Confirma `now` como paso nuevo si cambió (antes de acciones estructurales o al terminar una interacción).
    pub fn commit(&mut self, now: &str) {
        if now != self.cur {
            self.undo.push(std::mem::replace(&mut self.cur, now.to_string()));
            if self.undo.len() > MAX {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    pub fn can_undo(&self, now: &str) -> bool {
        !self.undo.is_empty() || now != self.cur
    }

    pub fn can_redo(&self, now: &str) -> bool {
        !self.redo.is_empty() && now == self.cur
    }

    /// Devuelve el estado a restaurar.
    pub fn undo(&mut self, now: &str) -> Option<String> {
        self.commit(now);
        let prev = self.undo.pop()?;
        self.redo.push(std::mem::replace(&mut self.cur, prev.clone()));
        Some(prev)
    }

    pub fn redo(&mut self, now: &str) -> Option<String> {
        self.commit(now);
        let next = self.redo.pop()?;
        self.undo.push(std::mem::replace(&mut self.cur, next.clone()));
        Some(next)
    }
}
